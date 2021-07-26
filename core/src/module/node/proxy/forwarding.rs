use crate::library::http::{forward_request, MatchableString, Responder};
use async_trait::async_trait;
use futures::Future;
use hyper::client::HttpConnector;
use hyper::http::{
    request::{Parts, Request},
    Response, Uri, Version,
};
use hyper::{Body, Client, StatusCode};
use std::convert::Infallible;
use std::net::IpAddr;

const SESSION_PREFIX: &str = "/session/";

pub struct ForwardingResponder {
    client: Client<HttpConnector>,
    identifier: String,
    authority: String,
    session_id_internal: String,
    session_id_external: String,
}

impl ForwardingResponder {
    pub fn new(
        identifier: String,
        authority: String,
        session_id_internal: String,
        session_id_external: String,
    ) -> Self {
        Self {
            client: Client::new(),
            identifier,
            authority,
            session_id_internal,
            session_id_external,
        }
    }

    #[inline]
    fn new_error_response(&self, message: &str) -> Response<Body> {
        // TODO Wrap the error in a WebDriver protocol compliant JSON error (and stack using the BlackboxError type)
        let error = format!("unable to forward request to webdriver: {}", message);

        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(error))
            .unwrap()
    }

    #[inline]
    fn match_request<'a>(&self, parts: &'a Parts) -> Option<&'a str> {
        let mut matchable = MatchableString::new(parts.uri.path());

        matchable.consume_prefix(SESSION_PREFIX)?;
        let identifier = matchable.consume_count(self.session_id_external.len())?;
        let remainder = matchable.current();

        if identifier == self.session_id_external {
            Some(remainder)
        } else {
            None
        }
    }

    #[inline]
    fn build_uri(&self, remainder: &str) -> Result<Uri, hyper::http::Error> {
        let path = format!(
            "{}{}{}",
            SESSION_PREFIX, self.session_id_internal, remainder
        );

        Uri::builder()
            .scheme("http")
            .authority(self.authority.as_str())
            .path_and_query(path)
            .build()
    }
}

#[async_trait]
impl Responder for ForwardingResponder {
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
        // Check if we have a matching path and build the target URI
        let matched_uri = self.match_request(&parts).map(|r| self.build_uri(r));

        let uri = match matched_uri {
            Some(Ok(uri)) => uri,
            Some(Err(e)) => return Ok(self.new_error_response(&e.to_string())),
            None => return next(parts, body, client_ip).await,
        };

        // Reconstruct the request and force HTTP/1.1 (because WebDrivers are ancient technology :P)
        let mut req = Request::from_parts(parts, body);
        *req.version_mut() = Version::HTTP_11;

        // Forward the request
        let forward_result =
            forward_request(&self.client, req, client_ip, &self.identifier, uri).await;

        // Handle potential errors
        let response = match forward_result {
            Ok(r) => r,
            Err(e) => self.new_error_response(&e.to_string()),
        };

        Ok(response)
    }
}
