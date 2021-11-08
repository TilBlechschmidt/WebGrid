use async_trait::async_trait;
use domain::event::SessionIdentifier;
use domain::webdriver::WebdriverErrorCode;
use domain::WebgridServiceDescriptor;
use futures::{Future, TryStreamExt};
use hyper::client::HttpConnector;
use hyper::http::{request::Parts, Request, Response, Version};
use hyper::{Body, Client};
use library::communication::discovery::{DiscoveredServiceEndpoint, ServiceDiscoverer};
use library::communication::BlackboxError;
use library::http::{
    forward_request, uri_with_authority, ForwardError, MatchableString, Responder,
};
use library::BoxedError;
use std::convert::Infallible;
use std::net::IpAddr;
use thiserror::Error;
use uuid::Uuid;

const SESSION_PREFIX: &str = "/session/";
pub const SESSION_ID_LENGTH: usize = 36; // Length of a UUID e.g. "7B43902E-7520-4AB3-AA1E-ACB4C52E6A6D"

#[derive(Debug, Error)]
enum SessionForwardingResponderError {
    #[error("endpoint discovery failed")]
    EndpointDiscoveryFailure(#[source] BoxedError),
    #[error("no endpoint available")]
    NoEndpoint,
    #[error("unable to construct destination URI")]
    URIConstructionFailed(#[source] hyper::http::Error),
    #[error("unable to forward request")]
    UnableToForward(#[source] ForwardError),
}

use SessionForwardingResponderError::*;

#[derive(Debug, Error)]
enum SessionForwardingError {
    #[error("unable to forward request to session {0}")]
    ForwardFailed(SessionIdentifier, #[source] SessionForwardingResponderError),
}

pub struct SessionForwardingResponder<D: ServiceDiscoverer<WebgridServiceDescriptor>> {
    client: Client<HttpConnector>,
    discoverer: D,
    identifier: String,
}

impl<D> SessionForwardingResponder<D>
where
    D: ServiceDiscoverer<WebgridServiceDescriptor> + Send + Sync,
    D::I: Send + Sync,
{
    pub fn new(identifier: String, discoverer: D) -> Self {
        Self {
            client: Client::builder().http2_only(true).build_http(),
            discoverer,
            identifier,
        }
    }

    #[inline]
    fn new_error_response(
        &self,
        id: SessionIdentifier,
        error: SessionForwardingResponderError,
    ) -> Response<Body> {
        let error = SessionForwardingError::ForwardFailed(id, error);
        let blackbox = BlackboxError::new(error);

        super::error::new_error_response(WebdriverErrorCode::UnknownError, blackbox)
    }

    #[inline]
    fn match_request(&self, parts: &Parts) -> Option<SessionIdentifier> {
        let mut matchable = MatchableString::new(parts.uri.path());

        matchable.consume_prefix(SESSION_PREFIX)?;
        let identifier = matchable.consume_count(SESSION_ID_LENGTH)?;

        // TODO This is a potentially slow operation.
        //      Resort to usage of &str for service discovery instead!
        match Uuid::parse_str(identifier) {
            Ok(id) => Some(id),
            // TODO Return the error to the client. Since /session/* is exclusive, we can assume
            //      that no other responder will handle the route so it makes sense to return a valid
            //      response containing an error with some details on why the request was rejected.
            Err(_e) => None,
        }
    }

    async fn discover_endpoint(
        &self,
        identifier: SessionIdentifier,
    ) -> Result<D::I, SessionForwardingResponderError> {
        let descriptor = WebgridServiceDescriptor::Node(identifier);
        let endpoint = self
            .discoverer
            .discover(descriptor)
            .try_next()
            .await
            .map_err(|e| EndpointDiscoveryFailure(e))?;
        endpoint.ok_or(NoEndpoint)
    }
}

#[async_trait]
impl<D> Responder for SessionForwardingResponder<D>
where
    D: ServiceDiscoverer<WebgridServiceDescriptor> + Send + Sync,
    D::I: Send + Sync,
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
        // Match the incoming request
        let identifier = match self.match_request(&parts) {
            Some(identifier) => identifier,
            None => return next(parts, body, client_ip).await,
        };

        // Search for an endpoint
        let endpoint = match self.discover_endpoint(identifier).await {
            Ok(endpoint) => endpoint,
            Err(e) => return Ok(self.new_error_response(identifier, e)),
        };

        // Reconstruct the request and force HTTP/2 for SPEEEEEED :P
        let mut req = Request::from_parts(parts, body);
        *req.version_mut() = Version::HTTP_2;

        // Build a URI from the parts
        let uri = match uri_with_authority(&req, &endpoint) {
            Ok(uri) => uri,
            Err(e) => return Ok(self.new_error_response(identifier, URIConstructionFailed(e))),
        };

        // Forward the request
        let forward_result =
            forward_request(&self.client, req, client_ip, &self.identifier, uri).await;

        // Handle potential errors
        let response = match forward_result {
            Ok(r) => r,
            Err(e) => {
                endpoint.flag_unreachable().await;
                self.new_error_response(identifier, UnableToForward(e))
            }
        };

        Ok(response)
    }
}
