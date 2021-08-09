use crate::domain::event::SessionIdentifier;
use crate::library::http::Responder;
use crate::library::storage::{storage_path, StorageBackend};
use async_trait::async_trait;
use futures::Future;
use hyper::http::{request::Parts, Method, Response, StatusCode};
use hyper::{body, Body};
use std::convert::Infallible;
use std::net::IpAddr;

pub struct StorageResponder<S: StorageBackend> {
    session_id: SessionIdentifier,
    storage: S,
}

impl<S> StorageResponder<S>
where
    S: StorageBackend,
{
    pub fn new(session_id: SessionIdentifier, storage: S) -> Self {
        Self {
            session_id,
            storage,
        }
    }

    #[inline]
    fn new_error_response(&self, message: &str) -> Response<Body> {
        let error = format!("unable to process request: {}", message);

        log::warn!("Unable to store video file: {}", message);

        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(error))
            .unwrap()
    }

    #[inline]
    fn new_response(&self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .unwrap()
    }
}

#[async_trait]
impl<S> Responder for StorageResponder<S>
where
    S: StorageBackend + Send + Sync,
{
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
        if client_ip.is_loopback() && parts.method == Method::PUT {
            match body::to_bytes(body).await {
                Ok(content) => {
                    let path = storage_path(self.session_id, parts.uri.path())
                        .to_string_lossy()
                        .into_owned();

                    if let Err(e) = self.storage.put_object(&path, &content).await {
                        Ok(self.new_error_response(&format!("object not writable {}", e)))
                    } else {
                        Ok(self.new_response())
                    }
                }
                Err(e) => Ok(self.new_error_response(&format!("body not readable {}", e))),
            }
        } else {
            next(parts, body, client_ip).await
        }
    }
}