//! HTTP healthcheck related functions
//!
//! Functions that are used to check if a HTTP endpoint is reachable

use hyper::{body, Client, Uri};
use log::{debug, trace};
use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
    Context as TelemetryContext,
};
use opentelemetry_http::HeaderInjector;
use redis::{aio::ConnectionLike, AsyncCommands, RedisResult};
use std::time::Duration;
use tokio::time::sleep;
use tokio::time::timeout;

use crate::libraries::tracing::global_tracer;

/// Sends HTTP requests to the specified URL until either a 200 OK response is received or the timeout is reached
pub async fn wait_for(url: &str, timeout_duration: Duration) -> Result<String, ()> {
    let client = Client::new();

    let url = url.parse::<Uri>().unwrap();

    let check_interval = Duration::from_millis(150);
    let request_timeout = Duration::from_millis(1000);
    let mut remaining_duration = timeout_duration;

    debug!("Awaiting 200 OK response from {}", url);

    loop {
        let span = global_tracer().start("Sending healthcheck request");
        let telemetry_context = TelemetryContext::current_with_span(span);

        let mut req = hyper::Request::new(hyper::Body::default());
        *req.uri_mut() = url.clone();

        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(
                &telemetry_context,
                &mut HeaderInjector(&mut req.headers_mut()),
            )
        });

        let request = client.request(req);

        trace!("Sending health-check request");
        let response = timeout(request_timeout, request).await;

        // Rust does not yet support boolean and let in the same IF statement. TODO Replace this once language support lands
        if let Ok(Ok(res)) = response {
            if res.status() == 200 {
                return match body::to_bytes(res.into_body()).await {
                    Ok(bytes) => {
                        Ok(String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()))
                    }
                    Err(_) => Ok("".to_string()),
                };
            }

            trace!("Received response with status != 200");
        } else {
            trace!("Unable to send request to node! {:?}", response);
        }

        if remaining_duration.as_secs() == 0 {
            debug!("Timeout while waiting for {}", url);
            return Err(());
        }

        telemetry_context.span().end();
        sleep(check_interval).await;
        remaining_duration -= check_interval;
    }
}

pub async fn wait_for_key<C: ConnectionLike + AsyncCommands>(
    key: &str,
    timeout_duration: Duration,
    con: &mut C,
) -> Result<(), ()> {
    let check_interval = Duration::from_millis(250);
    let mut remaining_duration = timeout_duration;

    debug!("Awaiting existence of redis key {}", key);

    loop {
        let result: RedisResult<bool> = con.exists(key).await;

        if let Ok(exists) = result {
            if exists {
                return Ok(());
            } else {
                trace!("Expected redis key does not exist yet");
            }
        } else {
            trace!("Unable to check existence of redis key! {:?}", result);
        }

        if remaining_duration.as_secs() == 0 {
            debug!("Timeout while waiting for redis key {}", key);
            return Err(());
        }

        sleep(check_interval).await;
        remaining_duration -= check_interval;
    }
}
