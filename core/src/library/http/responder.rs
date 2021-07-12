use async_trait::async_trait;
use http::Response;
use hyper::{http::request::Parts, Body};
use std::convert::Infallible;
use std::net::IpAddr;

/// Action taken by a [`Responder`]
pub enum ResponderResult {
    /// Request has been intercepted, processed, and a response created which may be sent to the client
    Intercepted(Result<Response<Body>, Infallible>),
    /// The inputs have been inspected and may be handled by the next [`Responder`] in the chain.
    /// Useful for intercepting or modifying incoming requests.
    Continue(Parts, Body, IpAddr),
}

/// Handler for incoming HTTP requests which may be chained
#[async_trait]
pub trait Responder {
    /// Executes the responder on the parts of a request
    async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult;
}

/// Simplifies chaining of calls to [`Responder::respond`]
///
/// Under the hood, it calls the `.respond` method of each passed responder in order.
/// If a responder returns [`ResponderResult::Intercepted`], the response is returned
/// using the `return` statement. Otherwise, the potentially modified request is passed
/// to the next responder in the chain.
///
/// ```no_run
///# use std::net::IpAddr;
///# use hyper::{Body, Response, StatusCode};
///# use http::request::Parts;
///# use webgrid::{library::http::{responder_chain, ResponderResult, Responder}};
///# use async_trait::async_trait;
///#
///# #[derive(Clone)]
///# struct StatusCodeResponder(Option<StatusCode>);
///#
///# #[async_trait]
///# impl Responder for StatusCodeResponder {
///#     #[inline]
///#     async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
///#         match self.0 {
///#             Some(status) => {
///#                 let response = Response::builder()
///#                     .status(status)
///#                     .body(Body::empty())
///#                     .unwrap();
///#
///#                 ResponderResult::Intercepted(response)
///#             }
///#             None => ResponderResult::Continue(parts, body, client_ip),
///#         }
///#     }
///# }
/// async fn respond(input_parts: Parts, input_body: Body, input_client_ip: IpAddr) -> Response<Body> {
///     let a = StatusCodeResponder(None);
///     let b = StatusCodeResponder(None);
///     let c = StatusCodeResponder(Some(StatusCode::OK));
///
///     responder_chain!(input_parts, input_body, input_client_ip, {
///         a,
///         b,
///         c
///     });
///
///     Response::builder()
///         .status(StatusCode::NOT_FOUND)
///         .body(Body::empty())
///         .unwrap()
/// }
/// ```
///
/// The above call will result in code similar to the below.
/// ```no_run
///# use std::net::IpAddr;
///# use hyper::{Body, Response, StatusCode};
///# use http::request::Parts;
///# use webgrid::{library::http::{responder_chain, ResponderResult, Responder}};
///# use async_trait::async_trait;
///#
///# #[derive(Clone)]
///# struct StatusCodeResponder(Option<StatusCode>);
///#
///# #[async_trait]
///# impl Responder for StatusCodeResponder {
///#     #[inline]
///#     async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
///#         match self.0 {
///#             Some(status) => {
///#                 let response = Response::builder()
///#                     .status(status)
///#                     .body(Body::empty())
///#                     .unwrap();
///#
///#                 ResponderResult::Intercepted(response)
///#             }
///#             None => ResponderResult::Continue(parts, body, client_ip),
///#         }
///#     }
///# }
/// async fn respond(input_parts: Parts, input_body: Body, input_client_ip: IpAddr) -> Response<Body> {
///     let a = StatusCodeResponder(None);
///     let b = StatusCodeResponder(None);
///     let c = StatusCodeResponder(Some(StatusCode::OK));
///
///     let parts = input_parts;
///     let body = input_body;
///     let ip = input_client_ip;
///
///     let (parts, body, ip) = match a.respond(parts, body, ip).await {
///         ResponderResult::Intercepted(response) => return response,
///         ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
///     };
///
///     let (parts, body, ip) = match b.respond(parts, body, ip).await {
///         ResponderResult::Intercepted(response) => return response,
///         ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
///     };
///
///     let (parts, body, ip) = match c.respond(parts, body, ip).await {
///         ResponderResult::Intercepted(response) => return response,
///         ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
///     };
///
///     drop(parts);
///     drop(body);
///     drop(ip);
///
///     Response::builder()
///         .status(StatusCode::NOT_FOUND)
///         .body(Body::empty())
///         .unwrap()
/// }
/// ```
#[macro_export]
macro_rules! responder_chain {
    ($parts:expr, $body:expr, $ip:expr, { $($responder:ident$(,)? )+ }) => {
        {
            let parts = $parts;
            let body = $body;
            let ip = $ip;

            $(
                #[allow(unused_variables)]
                let (parts, body, ip) = match $responder.respond(parts, body, ip).await {
                    ResponderResult::Intercepted(response) => return response,
                    ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
                };

            )+

            drop(parts);
            drop(body);
        }
    };
}

/// Combines a number of responder instances and creates a new hyper service function
/// through the use of [`make_service_fn`](hyper::service::make_service_fn). To allow
/// for concurrent access to the responders in the chain, they are wrapped in [`Arc`](std::sync::Arc) pointer types.
///
/// In case no responder matches the request, a `404 Not Found` response is sent to the client.
#[macro_export]
macro_rules! make_responder_chain_service_fn {
    ( $($responder:ident$(,)? )+ ) => {
        {
            use hyper::{Body, Response, http::StatusCode, server::conn::AddrStream, service::{make_service_fn, service_fn}};
            use std::{sync::Arc, convert::Infallible};
            use crate::{
                library::http::{Responder, ResponderResult},
                responder_chain,
            };

            paste::paste! {
                $(
                    let [<arc_ $responder>] = Arc::new($responder);
                )+

                make_service_fn(move |conn: &AddrStream| {
                    let addr = conn.remote_addr();

                    $(
                        let [<arc_ $responder>] = [<arc_ $responder>].clone();
                    )+

                    async move {
                        $(
                            let [<arc_ $responder>] = [<arc_ $responder>].clone();
                        )+

                        Ok::<_, Infallible>(service_fn(move |req| {
                            $(
                                let [<arc_ $responder>] = [<arc_ $responder>].clone();
                            )+

                            async move {
                                let (parts, body) = req.into_parts();

                                responder_chain!(parts, body, addr.ip(), {
                                    $(
                                        [<arc_ $responder>],
                                    )+
                                });

                                Ok(Response::builder()
                                    .status(StatusCode::NOT_FOUND)
                                    .body(Body::empty())
                                    .unwrap())
                            }
                        }))
                    }
                })
            }
        }
    };
}

pub use make_responder_chain_service_fn;
pub use responder_chain;
