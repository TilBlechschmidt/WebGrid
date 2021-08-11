use crate::domain::event::SessionIdentifier;
use crate::library::http::{MatchableString, Responder};
use crate::library::storage::{storage_path, StorageBackend};
use async_trait::async_trait;
use futures::Future;
use hyper::http::{request::Parts, Response, StatusCode};
use hyper::Body;
use std::convert::Infallible;
use std::net::IpAddr;
use uuid::Uuid;

use super::session::SESSION_ID_LENGTH;

const STORAGE_PREFIX: &str = "/storage/";

pub struct StorageResponder<S: StorageBackend> {
    storage: Option<S>,
}

impl<S> StorageResponder<S>
where
    S: StorageBackend,
{
    pub fn new(storage: Option<S>) -> Self {
        Self { storage }
    }

    #[inline]
    fn new_error_response(&self, message: &str, status: StatusCode) -> Response<Body> {
        // TODO Wrap the error in a WebDriver protocol compliant JSON error (and stack using the BlackboxError type)
        // TODO Add session ID to error message for easier debugging :)
        let error = format!("unable to forward request to session: {}", message);

        Response::builder()
            .status(status)
            .body(Body::from(error))
            .unwrap()
    }

    #[inline]
    fn match_request<'a>(&self, parts: &'a Parts) -> Option<(SessionIdentifier, &'a str)> {
        let mut matchable = MatchableString::new(parts.uri.path());

        matchable.consume_prefix(STORAGE_PREFIX)?;
        let identifier = matchable.consume_count(SESSION_ID_LENGTH)?;
        let remainder = matchable.current();

        // TODO This is a potentially slow operation.
        //      Resort to usage of &str for service discovery instead!
        match Uuid::parse_str(identifier) {
            Ok(id) => Some((id, remainder)),
            // TODO Return the error to the client. Since /storage/* is exclusive, we can assume
            //      that no other responder will handle the route so it makes sense to return a valid
            //      response containing an error with some details on why the request was rejected.
            Err(_e) => None,
        }
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
        let (session_id, filename) = match self.match_request(&parts) {
            Some(m) => m,
            None => return next(parts, body, client_ip).await,
        };

        let path = storage_path(session_id, filename);

        if let Some(storage) = &self.storage {
            match storage.get_object(&path.to_string_lossy()).await {
                Ok(object) => Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(object))
                    .unwrap()),
                Err(e) => Ok(self.new_error_response(&e.to_string(), StatusCode::NOT_FOUND)),
            }
        } else {
            Ok(self.new_error_response(
                "no storage backend configured",
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}
