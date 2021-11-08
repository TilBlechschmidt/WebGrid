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

/// Chains together a number of `Responder` implementations
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
        $last.respond($parts, $body, $ip, move |parts, _, ip| async move {
            use hyper::http::{Response, StatusCode};
            use tracing::warn;

            let method = parts.method.to_string();
            let path = parts.uri.path_and_query().map(|p| p.to_string()).unwrap_or_default();
            warn!(?ip, ?method, ?path, "No responder handled request");

            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("404 Not found.".into())
                .unwrap())
        })
    };
}

/// Combines a number of responder instances and creates a new hyper service function
/// through the use of [`make_service_fn`](hyper::service::make_service_fn). To allow
/// for concurrent access to the responders in the chain, they are wrapped in [`Arc`](std::sync::Arc) pointer types.
#[macro_export]
macro_rules! make_responder_chain_service_fn {
    ( $($responder:ident$(,)? )+ ) => {
        {
            use hyper::{server::conn::AddrStream, service::{make_service_fn, service_fn}};
            use std::{sync::Arc, convert::Infallible};
            use tracing::debug;

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

                                debug!(?ip, method = ?parts.method.to_string(), path = ?parts.uri.path_and_query().map(|p| p.to_string()).unwrap_or_default(), "Received request");

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
