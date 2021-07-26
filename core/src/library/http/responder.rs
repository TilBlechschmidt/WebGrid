use async_trait::async_trait;
use futures::Future;
use hyper::{
    http::{request::Parts, Response},
    Body,
};
use std::convert::Infallible;
use std::net::IpAddr;

/// Handler for incoming HTTP requests which may be chained
#[async_trait]
pub trait Responder {
    /// Executes the responder on the parts of a request
    async fn respond<F, Fut>(
        &self,
        parts: Parts,
        body: Body,
        client_ip: IpAddr,
        next: F,
    ) -> Result<Response<Body>, Infallible>
    where
        Fut: Future<Output = Result<Response<Body>, Infallible>> + Send,
        F: FnOnce(Parts, Body, IpAddr) -> Fut + Send;
}

/// TODO Apparently this thing has to be document
#[macro_export]
macro_rules! responder_chain {
    ($parts:expr, $body:expr, $ip:expr, { $first:ident, $($rest:tt)+ }) => {
        $first.respond($parts, $body, $ip, move |p, b, i| {
            async move {
                responder_chain!(p, b, i, { $($rest)+ }).await
            }
        })
    };

    ($parts:expr, $body:expr, $ip:expr, { $last:ident$(,)? }) => {
        $last.respond($parts, $body, $ip, move |_, _, _| async move {
            use hyper::http::{Response, StatusCode};

            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("404 Not found.".into())
                .unwrap())
        })
    };
}

// /// Simplifies chaining of calls to [`Responder::respond`]
// ///
// /// Under the hood, it calls the `.respond` method of each passed responder in order.
// /// If a responder returns [`ResponderResult::Intercepted`], the response is returned
// /// using the `return` statement. Otherwise, the potentially modified request is passed
// /// to the next responder in the chain.
// ///
// /// ```no_run
// ///# use std::net::IpAddr;
// ///# use hyper::{Body, Response, StatusCode};
// ///# use http::request::Parts;
// ///# use webgrid::{library::http::{responder_chain, ResponderResult, Responder}};
// ///# use async_trait::async_trait;
// ///# use std::convert::Infallible;
// ///#
// ///# #[derive(Clone)]
// ///# struct StatusCodeResponder(Option<StatusCode>);
// ///#
// ///# #[async_trait]
// ///# impl Responder for StatusCodeResponder {
// ///#     #[inline]
// ///#     async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
// ///#         match self.0 {
// ///#             Some(status) => {
// ///#                 let response = Response::builder()
// ///#                     .status(status)
// ///#                     .body(Body::empty())
// ///#                     .unwrap();
// ///#
// ///#                 ResponderResult::Intercepted(Ok(response))
// ///#             }
// ///#             None => ResponderResult::Continue(parts, body, client_ip),
// ///#         }
// ///#     }
// ///# }
// /// async fn respond(input_parts: Parts, input_body: Body, input_client_ip: IpAddr) -> Result<Response<Body>, Infallible> {
// ///     let a = StatusCodeResponder(None);
// ///     let b = StatusCodeResponder(None);
// ///     let c = StatusCodeResponder(Some(StatusCode::OK));
// ///
// ///     responder_chain!(input_parts, input_body, input_client_ip, {
// ///         a,
// ///         b,
// ///         c
// ///     });
// ///
// ///     Ok(Response::builder()
// ///         .status(StatusCode::NOT_FOUND)
// ///         .body(Body::empty())
// ///         .unwrap())
// /// }
// /// ```
// ///
// /// The above call will result in code similar to the below.
// /// ```no_run
// ///# use std::net::IpAddr;
// ///# use hyper::{Body, Response, StatusCode};
// ///# use http::request::Parts;
// ///# use webgrid::{library::http::{responder_chain, ResponderResult, Responder}};
// ///# use async_trait::async_trait;
// ///# use std::convert::Infallible;
// ///#
// ///# #[derive(Clone)]
// ///# struct StatusCodeResponder(Option<StatusCode>);
// ///#
// ///# #[async_trait]
// ///# impl Responder for StatusCodeResponder {
// ///#     #[inline]
// ///#     async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
// ///#         match self.0 {
// ///#             Some(status) => {
// ///#                 let response = Response::builder()
// ///#                     .status(status)
// ///#                     .body(Body::empty())
// ///#                     .unwrap();
// ///#
// ///#                 ResponderResult::Intercepted(Ok(response))
// ///#             }
// ///#             None => ResponderResult::Continue(parts, body, client_ip),
// ///#         }
// ///#     }
// ///# }
// /// async fn respond(input_parts: Parts, input_body: Body, input_client_ip: IpAddr) -> Result<Response<Body>, Infallible> {
// ///     let a = StatusCodeResponder(None);
// ///     let b = StatusCodeResponder(None);
// ///     let c = StatusCodeResponder(Some(StatusCode::OK));
// ///
// ///     let parts = input_parts;
// ///     let body = input_body;
// ///     let ip = input_client_ip;
// ///
// ///     let (parts, body, ip) = match a.respond(parts, body, ip).await {
// ///         ResponderResult::Intercepted(response) => return response,
// ///         ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
// ///     };
// ///
// ///     let (parts, body, ip) = match b.respond(parts, body, ip).await {
// ///         ResponderResult::Intercepted(response) => return response,
// ///         ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
// ///     };
// ///
// ///     let (parts, body, ip) = match c.respond(parts, body, ip).await {
// ///         ResponderResult::Intercepted(response) => return response,
// ///         ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
// ///     };
// ///
// ///     drop(parts);
// ///     drop(body);
// ///     drop(ip);
// ///
// ///     Ok(Response::builder()
// ///         .status(StatusCode::NOT_FOUND)
// ///         .body(Body::empty())
// ///         .unwrap())
// /// }
// /// ```
// #[macro_export]
// macro_rules! responder_chain {
//     ($parts:expr, $body:expr, $ip:expr, { $($responder:ident$(,)? )+ }) => {
//         {
//             let parts = $parts;
//             let body = $body;
//             let ip = $ip;

//             $(
//                 #[allow(unused_variables)]
//                 let (parts, body, ip) = match $responder.respond(parts, body, ip).await {
//                     ResponderResult::Intercepted(response) => return response,
//                     ResponderResult::Continue(parts, body, ip) => (parts, body, ip),
//                 };

//             )+

//             drop(parts);
//             drop(body);
//         }
//     };
// }

/// Combines a number of responder instances and creates a new hyper service function
/// through the use of [`make_service_fn`](hyper::service::make_service_fn). To allow
/// for concurrent access to the responders in the chain, they are wrapped in [`Arc`](std::sync::Arc) pointer types.
#[macro_export]
macro_rules! make_responder_chain_service_fn {
    ( $($responder:ident$(,)? )+ ) => {
        {
            use hyper::{server::conn::AddrStream, service::{make_service_fn, service_fn}};
            use std::{sync::Arc, convert::Infallible};
            use crate::{
                library::http::Responder,
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
                                let ip = addr.ip();

                                responder_chain!(parts, body, ip, {
                                    $(
                                        [<arc_ $responder>],
                                    )+
                                }).await
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
