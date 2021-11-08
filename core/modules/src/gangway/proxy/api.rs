use async_trait::async_trait;
use domain::WebgridServiceDescriptor;
use futures::{Future, TryStreamExt};
use hyper::client::HttpConnector;
use hyper::http::{request::Parts, Request, Response, StatusCode, Version};
use hyper::{Body, Client};
use library::communication::discovery::{DiscoveredServiceEndpoint, ServiceDiscoverer};
use library::http::{forward_request, uri_with_authority, Responder};
use library::BoxedError;
use std::convert::Infallible;
use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Error)]
enum ApiForwardingError {
    #[error("no endpoint available for API")]
    NoEndpoint,
}

pub struct ApiForwardingResponder<D: ServiceDiscoverer<WebgridServiceDescriptor>> {
    client: Client<HttpConnector>,
    discoverer: D,
    identifier: String,
}

impl<D> ApiForwardingResponder<D>
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
        let error = format!("unable to forward request to API: {}", message);

        Response::builder()
            .status(status)
            .body(Body::from(error))
            .unwrap()
    }

    async fn discover_endpoint(&self) -> Result<D::I, BoxedError> {
        let descriptor = WebgridServiceDescriptor::Api;
        let endpoint = self.discoverer.discover(descriptor).try_next().await?;
        endpoint.ok_or_else(|| ApiForwardingError::NoEndpoint.into())
    }
}

#[async_trait]
impl<D> Responder for ApiForwardingResponder<D>
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
        _next: F,
    ) -> Result<Response<Body>, Infallible>
    where
        Fut: Future<Output = Result<Response<Body>, Infallible>> + Send,
        F: FnOnce(Parts, Body, IpAddr) -> Fut + Send,
    {
        // No need to match anything, the API is a catch-all last-resort handler

        // Search for an endpoint
        let endpoint = match self.discover_endpoint().await {
            Ok(endpoint) => endpoint,
            Err(e) => return Ok(self.new_error_response(&e.to_string(), StatusCode::BAD_GATEWAY)),
        };

        // Reconstruct the request and force HTTP/2 for SPEEEEEED :P
        let mut req = Request::from_parts(parts, body);
        *req.version_mut() = Version::HTTP_2;

        // Build a URI from the parts
        let uri = match uri_with_authority(&req, &endpoint) {
            Ok(uri) => uri,
            Err(e) => {
                return Ok(
                    self.new_error_response(&e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                )
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

        Ok(response)
    }
}
