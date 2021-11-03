use crate::domain::event::SessionClientMetadata;
use crate::harness::HeartStone;
use crate::library::http::Responder;
use async_trait::async_trait;
use futures::Future;
use hyper::body::to_bytes;
use hyper::{
    http::{request::Parts, Method, Response},
    Body,
};
use serde_json::json;
use std::convert::Infallible;
use std::net::IpAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tracing::debug;

#[derive(Debug, Error)]
enum MetadataExtensionInterceptorError {
    #[error("collecting request bytes failed")]
    StreamingError(#[from] hyper::Error),
    #[error("failed to deserialize metadata")]
    DeserializationError(#[from] serde_json::Error),
    #[error("submitting metadata failed")]
    MetadataSendFailed(#[from] tokio::sync::mpsc::error::SendError<SessionClientMetadata>),
}

pub struct MetadataExtensionInterceptor {
    metadata_tx: UnboundedSender<SessionClientMetadata>,
    session_id: String,
    heart_stone: Arc<Mutex<HeartStone>>,
}

impl MetadataExtensionInterceptor {
    pub fn new(
        metadata_tx: UnboundedSender<SessionClientMetadata>,
        heart_stone: HeartStone,
        session_id: String,
    ) -> Self {
        Self {
            metadata_tx,
            session_id,
            heart_stone: Arc::new(Mutex::new(heart_stone)),
        }
    }

    async fn handle_body(&self, body: Body) -> Result<(), MetadataExtensionInterceptorError> {
        let bytes = to_bytes(body).await?;
        let metadata: SessionClientMetadata = serde_json::from_slice(&bytes)?;

        debug!(?metadata, "Received metadata from client");
        self.metadata_tx.send(metadata)?;

        Ok(())
    }
}

#[async_trait]
impl Responder for MetadataExtensionInterceptor {
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
        // Reset the lifetime
        self.heart_stone.lock().await.reset_lifetime().await;

        // Verify the method is POST
        if parts.method != Method::POST {
            return next(parts, body, client_ip).await;
        }

        // Verify the path matches the metadata extension url
        if !parts
            .uri
            .path()
            .eq_ignore_ascii_case(&format!("/session/{}/webgrid/metadata", self.session_id))
        {
            return next(parts, body, client_ip).await;
        }

        // Handle the metadata and json-ify the result
        let response_value = match self.handle_body(body).await {
            Ok(_) => {
                json!({ "status": "success" })
            }
            Err(e) => {
                json!({
                    "status": "error",
                    "error": e.to_string()
                })
            }
        };

        // Build a json response and send it
        let response = serde_json::to_string(&response_value).unwrap_or_else(|_| "{}".into());
        Ok(Response::builder().body(response.into()).unwrap())
    }
}
