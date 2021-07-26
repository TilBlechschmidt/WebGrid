use super::super::{SessionCreationCommunicationHandle, StatusResponse};
use crate::domain::event::{SessionCreatedNotification, SessionOperationalNotification};
use crate::domain::webdriver::{
    RawCapabilitiesRequest, SessionCreateResponse, SessionCreateResponseValue,
    SessionCreationRequest,
};
use crate::library::http::Responder;
use async_trait::async_trait;
use futures::Future;
use hyper::http::{request::Parts, Method, Response, StatusCode};
use hyper::{body, Body};
use std::convert::Infallible;
use std::net::IpAddr;
use tokio::sync::oneshot;
use uuid::Uuid;

const SESSION_CREATION_PATH: &str = "/session";

pub struct SessionCreationResponder {
    handle: SessionCreationCommunicationHandle,
}

impl SessionCreationResponder {
    pub fn new(handle: SessionCreationCommunicationHandle) -> Self {
        Self { handle }
    }

    #[inline]
    fn new_error_response(&self, id: &str, message: &str, status: StatusCode) -> Response<Body> {
        // TODO Wrap the error in a WebDriver protocol compliant JSON error (and stack using the BlackboxError type)
        let error = format!("unable to create session {}: {}", id, message);

        Response::builder()
            .status(status)
            .body(Body::from(error))
            .unwrap()
    }

    #[inline]
    fn new_success_response(&self, notification: SessionOperationalNotification) -> Response<Body> {
        let capabilities = match serde_json::from_str(&notification.actual_capabilities) {
            Ok(value) => value,
            Err(e) => {
                return self.new_error_response(
                    &notification.id.to_string(),
                    &e.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };

        let response = SessionCreateResponse {
            value: SessionCreateResponseValue {
                session_id: notification.id.to_string(),
                // TODO Handle this unwrap!
                capabilities,
            },
        };

        let serialized_response = match serde_json::to_string(&response) {
            Ok(serialized) => serialized,
            Err(e) => {
                return self.new_error_response(
                    &notification.id.to_string(),
                    &e.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        };

        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(serialized_response))
            .unwrap()
    }
}

#[async_trait]
impl Responder for SessionCreationResponder {
    #[inline]
    async fn respond<F, Fut>(
        &self,
        parts: Parts,
        body: Body,
        client_ip: IpAddr,
        next: F,
    ) -> Result<Response<Body>, Infallible>
    where
        Fut: Future<Output = Result<Response<Body>, Infallible>> + Send,
        F: FnOnce(Parts, Body, IpAddr) -> Fut + Send,
    {
        // Match the method and path or short-circuit
        let method_matches = parts.method == Method::POST;
        let path_matches = parts.uri.path().eq_ignore_ascii_case(SESSION_CREATION_PATH);

        if !(method_matches && path_matches) {
            return next(parts, body, client_ip).await;
        }

        // TODO Limit the number of pending requests by using a Semaphore. This prevents DDoS attacks (somewhat) and limits the number of open connections!

        // Generate/extract necessary data
        // TODO Ensure that a content-length header is set and has a value lower than a certain threshold to prevent OOM attacks!
        let bytes = match body::to_bytes(body).await {
            Ok(bytes) => bytes,
            Err(e) => {
                return Ok(self.new_error_response(
                    "-",
                    &e.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        };
        let capabilities = match serde_json::from_slice::<SessionCreationRequest>(&bytes)
            .map(|r| serde_json::to_string(&r.capabilities))
        {
            Ok(Ok(raw)) => RawCapabilitiesRequest::new(raw),
            Ok(Err(e)) => {
                return Ok(self.new_error_response(
                    "-",
                    &e.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
            Err(e) => {
                return Ok(self.new_error_response(
                    "-",
                    &e.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        };

        let id = Uuid::new_v4();
        let notification = SessionCreatedNotification { id, capabilities };

        // Create a channel for receiving the status and register it
        let (status_tx, status_rx) = oneshot::channel();
        self.handle.status_listeners.lock().await.put(id, status_tx);

        // Send the notification and handle potential errors
        if let Err(e) = self.handle.creation_tx.send(notification) {
            return Ok(self.new_error_response(
                &id.to_string(),
                &e.to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }

        // Wait for either a failed or successful startup
        match status_rx.await {
            Ok(StatusResponse::Operational(notification)) => {
                Ok(self.new_success_response(notification))
            }
            Ok(StatusResponse::Failed(notification)) => Ok(self.new_error_response(
                &id.to_string(),
                &notification.cause.to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
            Err(_) => Ok(self.new_error_response(
                &id.to_string(),
                "gangway has exceeded the maximum pending request limit",
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        }
    }
}
