use super::super::{SessionCreationCommunicationHandle, StatusResponse};
use async_trait::async_trait;
use domain::event::{
    SessionCreatedNotification, SessionIdentifier, SessionOperationalNotification,
};
use domain::webdriver::{
    RawCapabilitiesRequest, SessionCreateResponse, SessionCreateResponseValue,
    SessionCreationRequest, WebdriverErrorCode,
};
use futures::Future;
use hyper::http::{request::Parts, Method, Response, StatusCode};
use hyper::{body, Body};
use library::communication::BlackboxError;
use library::http::Responder;
use std::convert::Infallible;
use std::net::IpAddr;
use thiserror::Error;
use tokio::sync::oneshot;
use uuid::Uuid;

const SESSION_CREATION_PATH: &str = "/session";

#[derive(Debug, Error)]
enum SessionCreationResponderError {
    #[error("unable to parse actual capabilities")]
    InvalidActualCapabilities(#[source] serde_json::Error),
    #[error("unable to serialize response")]
    ResponseSerializationFailed(#[source] serde_json::Error),
    #[error("request body invalid")]
    InvalidRequestBody(#[source] hyper::Error),
    #[error("request content is invalid")]
    InvalidRequestContent(#[source] serde_json::Error),
    #[error("unable to submit session creation request")]
    CreationNotificationPublishFailed,
    #[error("session did not start up properly")]
    SessionStartupFailed(#[source] BlackboxError),
    #[error("pending request limit exceeded")]
    PendingRequestLimitExceeded,
}

use SessionCreationResponderError::*;

#[derive(Debug, Error)]
enum SessionCreationError {
    #[error("unable to create session {0}")]
    SessionCreationFailed(SessionIdentifier, #[source] SessionCreationResponderError),
}

pub struct SessionCreationResponder {
    handle: SessionCreationCommunicationHandle,
}

impl SessionCreationResponder {
    pub fn new(handle: SessionCreationCommunicationHandle) -> Self {
        Self { handle }
    }

    #[inline]
    fn new_error_response(
        &self,
        id: SessionIdentifier,
        error: SessionCreationResponderError,
    ) -> Response<Body> {
        let error = SessionCreationError::SessionCreationFailed(id, error);
        let blackbox = BlackboxError::new(error);

        super::error::new_error_response(WebdriverErrorCode::SessionNotCreated, blackbox)
    }

    #[inline]
    fn new_success_response(&self, notification: SessionOperationalNotification) -> Response<Body> {
        let capabilities = match serde_json::from_str(&notification.actual_capabilities) {
            Ok(value) => value,
            Err(e) => {
                return self.new_error_response(notification.id, InvalidActualCapabilities(e))
            }
        };

        let response = SessionCreateResponse {
            value: SessionCreateResponseValue {
                session_id: notification.id.to_string(),
                capabilities,
            },
        };

        let serialized_response = match serde_json::to_string(&response) {
            Ok(serialized) => serialized,
            Err(e) => {
                return self.new_error_response(notification.id, ResponseSerializationFailed(e))
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
        let id = Uuid::new_v4();
        // TODO Ensure that a content-length header is set and has a value lower than a certain threshold to prevent OOM attacks!
        let bytes = match body::to_bytes(body).await {
            Ok(bytes) => bytes,
            Err(e) => return Ok(self.new_error_response(id, InvalidRequestBody(e))),
        };
        let capabilities = match serde_json::from_slice::<SessionCreationRequest>(&bytes)
            .map(|r| serde_json::to_string(&r.capabilities))
        {
            Ok(Ok(raw)) => RawCapabilitiesRequest::new(raw),
            Ok(Err(e)) => return Ok(self.new_error_response(id, InvalidRequestContent(e))),
            Err(e) => return Ok(self.new_error_response(id, InvalidRequestContent(e))),
        };

        let notification = SessionCreatedNotification { id, capabilities };

        // Create a channel for receiving the status and register it
        let (status_tx, status_rx) = oneshot::channel();
        self.handle.status_listeners.lock().await.put(id, status_tx);

        // Send the notification and handle potential errors
        #[allow(clippy::question_mark)]
        if self.handle.creation_tx.send(notification).is_err() {
            return Ok(self.new_error_response(id, CreationNotificationPublishFailed));
        }

        // Wait for either a failed or successful startup
        match status_rx.await {
            Ok(StatusResponse::Operational(notification)) => {
                Ok(self.new_success_response(notification))
            }
            Ok(StatusResponse::Failed(notification)) => Ok(self.new_error_response(
                id,
                SessionStartupFailed(BlackboxError::new(notification.reason)),
            )),
            Err(_) => Ok(self.new_error_response(id, PendingRequestLimitExceeded)),
        }
    }
}
