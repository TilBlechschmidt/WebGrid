use super::super::{SessionCreationCommunicationHandle, StatusResponse};
use crate::domain::event::{SessionCreatedNotification, SessionOperationalNotification};
use crate::domain::webdriver::{
    RawCapabilitiesRequest, SessionCreateResponse, SessionCreateResponseValue,
    SessionCreationRequest,
};
use crate::library::http::{Responder, ResponderResult};
use async_trait::async_trait;
use hyper::http::{request::Parts, Method, Response, StatusCode};
use hyper::{body, Body};
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
    fn new_error_response(&self, message: &str, status: StatusCode) -> Response<Body> {
        // TODO Wrap the error in a WebDriver protocol compliant JSON error (and stack using the BlackboxError type)
        let error = format!("unable to create session: {}", message);

        Response::builder()
            .status(status)
            .body(Body::from(error))
            .unwrap()
    }

    #[inline]
    fn new_success_response(&self, notification: SessionOperationalNotification) -> Response<Body> {
        let capabilities = match serde_json::to_value(notification.actual_capabilities) {
            Ok(value) => value,
            Err(e) => {
                return self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
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
                return self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
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
    async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
        // Match the method and path or short-circuit
        let method_matches = parts.method == Method::POST;
        let path_matches = parts.uri.path().eq_ignore_ascii_case(SESSION_CREATION_PATH);

        if !(method_matches && path_matches) {
            return ResponderResult::Continue(parts, body, client_ip);
        }

        // TODO Limit the number of pending requests by using a Semaphore. This prevents DDoS attacks (somewhat) and limits the number of open connections!

        // Generate/extract necessary data
        // TODO Ensure that a content-length header is set and has a value lower than a certain threshold to prevent OOM attacks!
        let bytes = match body::to_bytes(body).await {
            Ok(bytes) => bytes,
            Err(e) => {
                return ResponderResult::Intercepted(Ok(
                    self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                ))
            }
        };
        let capabilities = match serde_json::from_slice::<SessionCreationRequest>(&bytes)
            .map(|r| serde_json::to_string(&r.capabilities))
        {
            Ok(Ok(raw)) => RawCapabilitiesRequest::new(raw),
            Ok(Err(e)) => {
                return ResponderResult::Intercepted(Ok(
                    self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                ))
            }
            Err(e) => {
                return ResponderResult::Intercepted(Ok(
                    self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
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
            return ResponderResult::Intercepted(Ok(
                self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            ));
        }

        // Wait for either a failed or successful startup
        match status_rx.await {
            Ok(StatusResponse::Operational(notification)) => {
                ResponderResult::Intercepted(Ok(self.new_success_response(notification)))
            }
            Ok(StatusResponse::Failed(notification)) => ResponderResult::Intercepted(Ok(self
                .new_error_response(
                    &notification.cause.to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))),
            Err(_) => ResponderResult::Intercepted(Ok(self.new_error_response(
                "gangway has exceeded the maximum pending request limit",
                StatusCode::INTERNAL_SERVER_ERROR,
            ))),
        }
    }
}
