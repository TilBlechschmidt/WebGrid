use crate::domain::event::SessionIdentifier;
use crate::domain::WebgridServiceDescriptor;
use crate::library::communication::discovery::{DiscoveredServiceEndpoint, ServiceDiscoverer};
use crate::library::http::{
    forward_request, uri_with_authority, MatchableString, Responder, ResponderResult,
};
use crate::library::BoxedError;
use async_trait::async_trait;
use futures::TryStreamExt;
use hyper::client::HttpConnector;
use hyper::http::{request::Parts, Request, Response, StatusCode, Version};
use hyper::{Body, Client};
use std::net::IpAddr;
use thiserror::Error;
use uuid::Uuid;

const SESSION_PREFIX: &str = "/session/";
const SESSION_ID_LENGTH: usize = 36; // Length of a UUID e.g. "7B43902E-7520-4AB3-AA1E-ACB4C52E6A6D"

#[derive(Debug, Error)]
enum SessionForwardingError {
    #[error("no endpoint available")]
    NoEndpoint,
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
    fn new_error_response(&self, message: &str, status: StatusCode) -> Response<Body> {
        // TODO Wrap the error in a WebDriver protocol compliant JSON error (and stack using the BlackboxError type)
        let error = format!("unable to forward request to session: {}", message);

        Response::builder()
            .status(status)
            .body(Body::from(error))
            .unwrap()
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

    async fn discover_endpoint(&self, identifier: SessionIdentifier) -> Result<D::I, BoxedError> {
        let descriptor = WebgridServiceDescriptor::Node(identifier);
        let endpoint = self.discoverer.discover(descriptor).try_next().await?;
        endpoint.ok_or_else(|| SessionForwardingError::NoEndpoint.into())
    }
}

#[async_trait]
impl<D> Responder for SessionForwardingResponder<D>
where
    D: ServiceDiscoverer<WebgridServiceDescriptor> + Send + Sync,
    D::I: Send + Sync,
{
    async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
        // Match the incoming request
        let identifier = match self.match_request(&parts) {
            Some(identifier) => identifier,
            None => return ResponderResult::Continue(parts, body, client_ip),
        };

        // Search for an endpoint
        let endpoint = match self.discover_endpoint(identifier).await {
            Ok(endpoint) => endpoint,
            Err(e) => {
                return ResponderResult::Intercepted(Ok(
                    self.new_error_response(&e.to_string(), StatusCode::BAD_GATEWAY)
                ))
            }
        };

        // Reconstruct the request and force HTTP/2 for SPEEEEEED :P
        let mut req = Request::from_parts(parts, body);
        *req.version_mut() = Version::HTTP_2;

        // Build a URI from the parts
        let uri = match uri_with_authority(&req, &endpoint) {
            Ok(uri) => uri,
            Err(e) => {
                return ResponderResult::Intercepted(Ok(
                    self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                ))
            }
        };

        // Forward the request
        let forward_result =
            forward_request(&self.client, req, client_ip, &self.identifier, uri).await;

        // Handle potential errors
        let response = match forward_result {
            Ok(r) => r,
            Err(e) => {
                endpoint.flag_unreachable().await;
                self.new_error_response(&e.to_string(), StatusCode::BAD_GATEWAY)
            }
        };

        ResponderResult::Intercepted(Ok(response))
    }
}
