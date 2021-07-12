//! Functions for forwarding HTTP requests to remote endpoints

use hyper::client::HttpConnector;
use hyper::http::{
    header::{
        Entry, HeaderName, InvalidHeaderValue, ToStrError, CONNECTION, FORWARDED, HOST,
        PROXY_AUTHENTICATE, PROXY_AUTHORIZATION, TE, TRAILER, TRANSFER_ENCODING, UPGRADE, VIA,
    },
    HeaderMap, HeaderValue, Request, Response, Uri,
};
use hyper::{Body, Client};
use lazy_static::lazy_static;
use std::net::IpAddr;
use thiserror::Error;

lazy_static! {
    static ref HOP_HEADERS: [HeaderName; 7] = [
        CONNECTION,
        // HeaderName::from_static("Keep-Alive"),
        PROXY_AUTHENTICATE,
        PROXY_AUTHORIZATION,
        TE,
        TRAILER,
        TRANSFER_ENCODING,
        UPGRADE,
    ];
}

/// HTTP reverse proxy error
#[derive(Debug, Error)]
pub enum ForwardError {
    /// Unable to construct proxy header value
    #[error("unable to construct proxy header value")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    /// Received non-ASCII proxy header (via/forwarded)
    #[error("received non-ASCII proxy header (via/forwarded)")]
    InvalidHeaderInput(#[from] ToStrError),
    /// HTTP connection failed
    #[error("http connection failed")]
    ConnectionFailed(#[from] hyper::Error),
}

macro_rules! append_header_value {
    ($req:expr, $key:expr, $value:expr) => {
        match $req.headers_mut().entry($key) {
            Entry::Vacant(entry) => {
                entry.insert($value.parse()?);
            }
            Entry::Occupied(mut entry) => {
                let existing_value = entry.get().to_str()?;
                let new_value = format!("{}, {}", existing_value, $value);

                entry.insert(new_value.parse()?);
            }
        }
    };
}

#[inline]
fn add_proxy_headers<B>(
    req: &mut Request<B>,
    ip: &IpAddr,
    proxy_identifier: &str,
) -> Result<(), ForwardError> {
    let proto = req.version();
    let host = req
        .headers()
        .get(HOST)
        .map(|v| v.to_str().ok())
        .flatten()
        .map(|host| format!(";host={}", host))
        .unwrap_or_default();

    // Forwarded: for;proto;host
    let forwarded_value = format!("for={};proto={:?}{}", ip, proto, host);
    append_header_value!(req, FORWARDED, forwarded_value);

    // Via: HTTP/2 webgrid-proxy-{options.id}
    let via_value = format!("{:?} webgrid-proxy-{}", proto, proxy_identifier);
    append_header_value!(req, VIA, via_value);

    Ok(())
}

#[inline]
fn strip_hop_headers(headers: &mut HeaderMap<HeaderValue>) {
    HOP_HEADERS.iter().for_each(|key| {
        headers.remove(key);
    });
}

#[inline]
fn translate_request<B>(
    ip: IpAddr,
    mut req: Request<B>,
    target: Uri,
    proxy_identifier: &str,
) -> Result<Request<B>, ForwardError> {
    *req.uri_mut() = target;

    strip_hop_headers(req.headers_mut());
    add_proxy_headers(&mut req, &ip, proxy_identifier)?;

    Ok(req)
}

/// Extracts the [`Uri`] from a request and replaces the authority with the provided value
///
/// Additionally, the scheme will be fixed to `http`.
#[inline]
pub fn uri_with_authority<B>(req: &Request<B>, authority: &str) -> Result<Uri, hyper::http::Error> {
    let mut uri = Uri::builder().scheme("http").authority(authority);

    if let Some(p_and_q) = req.uri().path_and_query() {
        uri = uri.path_and_query(p_and_q.clone());
    }

    uri.build()
}

/// Takes an incoming request and forwards it to a remote target
///
/// Additional information provided (e.g. client IP, proxy identifier) will be attached to the
/// requests [`VIA`] and [`FORWARDED`] header.
#[inline]
pub async fn forward_request(
    client: &Client<HttpConnector>,
    req: Request<Body>,
    source_ip: IpAddr,
    proxy_identifier: &str,
    target: Uri,
) -> Result<Response<Body>, ForwardError> {
    let upstream = target.host().map(|h| h.to_owned());
    let req = translate_request(source_ip, req, target, proxy_identifier)?;

    match client.request(req).await {
        Ok(mut res) => {
            strip_hop_headers(res.headers_mut());
            Ok(res)
        }
        Err(e) => {
            log::error!(
                "Failed to fulfill request to '{}': {}",
                upstream.unwrap_or_default(),
                e
            );

            Err(e.into())
        }
    }
}
