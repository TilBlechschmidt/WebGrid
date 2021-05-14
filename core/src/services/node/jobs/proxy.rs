use super::super::Context;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::{helpers::keys, tracing::global_tracer};
use crate::libraries::{lifecycle::HeartStone, resources::RedisResource};
use crate::with_shared_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use hyper::{
    body,
    client::HttpConnector,
    http::request::Parts,
    service::{make_service_fn, service_fn},
    Body, Client as HttpClient, Error as HyperError, Method, Request, Response, Server, Version,
};
use jatsl::{Job, TaskManager};
use lazy_static::lazy_static;
use log::{info, trace, warn};
use opentelemetry::{
    global,
    trace::{Span, StatusCode, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
use opentelemetry_http::HeaderExtractor;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use regex::Regex;
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

lazy_static! {
    static ref SESSION_RE: Regex = Regex::new(r"/session/(?P<sid>[^/]*)(?:/(?P<op>.+))?").unwrap();
}

pub struct ProxyJob {
    client: HttpClient<HttpConnector>,
    internal_session_id: String,
    heart_stone: HeartStone,
    port: u16,
}

#[derive(Clone)]
struct RequestContext {
    internal_session_id: String,
    external_session_id: String,
    client: HttpClient<HttpConnector>,
    heart_stone: HeartStone,
    context: Context,
    driver_port: u16,
    redis: Arc<Mutex<RedisResource<MultiplexedConnection>>>,
}

#[async_trait]
impl Job for ProxyJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let redis = Arc::new(Mutex::new(with_shared_redis_resource!(manager)));
        let request_context = RequestContext {
            internal_session_id: self.internal_session_id.clone(),
            external_session_id: manager.context.id.to_string(),
            client: self.client.clone(),
            heart_stone: self.heart_stone.clone(),
            context: manager.context.clone(),
            driver_port: manager.context.options.driver_port,
            redis,
        };

        let make_svc = make_service_fn(|_conn| {
            let request_context = request_context.clone();

            async move {
                Ok::<_, HyperError>(service_fn(move |req| {
                    ProxyJob::handle(req, request_context.clone())
                }))
            }
        });

        let addr = ([0, 0, 0, 0], self.port).into();
        let server = Server::bind(&addr).serve(make_svc);
        let graceful = server.with_graceful_shutdown(manager.termination_signal());

        info!("Listening on {}", addr);
        manager.ready().await;
        graceful.await?;

        Ok(())
    }
}

impl ProxyJob {
    pub fn new(port: u16, internal_session_id: String, heart_stone: HeartStone) -> Self {
        Self {
            client: HttpClient::new(),
            internal_session_id,
            heart_stone,
            port,
        }
    }

    async fn handle_driver_response(
        driver_response: Response<Body>,
        path: String,
        method: Method,
        mut ctx: RequestContext,
    ) -> Result<Response<Body>, HyperError> {
        // Split the response body apart and read it for logging and termination checks
        let response_status = driver_response.status();
        let (parts, body) = driver_response.into_parts();

        let body = match body::to_bytes(body).await {
            Ok(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()),
            Err(_) => "".to_string(),
        };

        trace!(
            "<- {} {} => {}; body: '{}'",
            method,
            path,
            response_status,
            body.replace("\n", "")
        );

        // Evaluate if this is a termination request
        let is_session_delete_request =
            method == Method::DELETE && path == format!("/session/{}", ctx.external_session_id);
        let is_window_delete_request = method == Method::DELETE
            && path == format!("/session/{}/window", ctx.external_session_id);

        let session_closed = if is_window_delete_request {
            lazy_static! {
                static ref EMPTY_VALUE_RE: Regex = Regex::new(r#""value": ?\[\]"#).unwrap();
            }

            EMPTY_VALUE_RE.is_match(&body)
        } else {
            is_session_delete_request
        };

        // Terminate the service if requested
        if session_closed {
            warn!("Session closed by downstream");
            ctx.heart_stone
                .kill("Session closed by downstream".to_string())
                .await;
        }

        Ok(Response::from_parts(parts, Body::from(body)))
    }

    async fn handle(
        mut req: Request<Body>,
        mut ctx: RequestContext,
    ) -> Result<Response<Body>, HyperError> {
        // Extract tracing context
        let parent_cx = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(req.headers()))
        });
        let span = global_tracer().start_with_context("Forward to driver", parent_cx);

        // Reset the lifetime
        ctx.heart_stone.reset_lifetime().await;

        // Translate the request path
        span.add_event("Translating request path".to_string(), vec![]);
        let req_path = req
            .uri()
            .path_and_query()
            .map(|x| x.as_str().to_owned())
            .unwrap_or_else(|| "".to_string());
        let path = match SESSION_RE.captures(&req_path) {
            Some(captures) => {
                let session_id = &captures["sid"];

                if session_id == ctx.external_session_id {
                    match captures.name("op") {
                        Some(operation) => {
                            format!(
                                "/session/{}/{}",
                                ctx.internal_session_id,
                                operation.as_str()
                            )
                        }
                        None => format!("/session/{}", ctx.internal_session_id),
                    }
                } else {
                    // TODO Treat this as an unauthorized request and return a 421
                    req_path.to_string()
                }
            }
            None => req_path.to_string(),
        };

        // Overwrite the original path with the translated one and downgrade to HTTP/1.1
        let driver_addr: SocketAddr = ([127, 0, 0, 1], ctx.driver_port).into();
        let req_method = req.method().clone();
        let uri_string = format!("http://{}{}", driver_addr, path);
        let uri = uri_string.parse().unwrap();
        *req.uri_mut() = uri;
        *req.version_mut() = Version::HTTP_11;

        // Split the request body apart and read it for logging and intercepting
        span.add_event("Interpreting request body".to_string(), vec![]);
        let (req_parts, req_body) = req.into_parts();
        let req_body = match body::to_bytes(req_body).await {
            Ok(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()),
            Err(_) => "".to_string(),
        };

        trace!(
            "-> {} {} -> {}; body: '{}'",
            req_method,
            path,
            uri_string,
            req_body.replace("\n", "")
        );

        // Run any special handling for the request (e.g. WebVTT cookies)
        span.add_event("Running local intercepts".to_string(), vec![]);
        ProxyJob::run_local_intercepts(&req_parts, &path, &req_body, &ctx).await;

        // Rebuild the request and create a request object
        let rebuild_req = Request::from_parts(req_parts, Body::from(req_body));
        let proxy_request = ctx.client.request(rebuild_req);

        // Dispatch the request
        let telemetry_context = TelemetryContext::current_with_span(span);
        let driver_span =
            global_tracer().start_with_context("WebDriver internal", telemetry_context.clone());
        match proxy_request.await {
            Ok(driver_response) => {
                driver_span.end();
                ProxyJob::handle_driver_response(driver_response, req_path, req_method, ctx).await
            }
            Err(driver_response) => {
                driver_span.set_status(StatusCode::Error, driver_response.to_string());
                warn!("Upstream error {}", driver_response);
                Err(driver_response)
            }
        }
    }

    async fn run_local_intercepts(parts: &Parts, path: &str, body: &str, ctx: &RequestContext) {
        if parts.method == Method::POST && path.ends_with("/cookie") {
            if let Ok(cookie_req) = serde_json::from_str::<CookieRequest>(body) {
                ProxyJob::handle_cookie_request(cookie_req, ctx).await;
            }
        }
    }

    async fn handle_cookie_request(request: CookieRequest, ctx: &RequestContext) {
        if request.cookie.name == "webgrid:message" {
            if let Some(webvtt) = &mut *ctx.context.webvtt.lock().await {
                webvtt.write(request.cookie.value).await.ok();
            }
        } else if let Some(key) = request
            .cookie
            .name
            .strip_prefix("webgrid:metadata.session:")
        {
            let value = request.cookie.value;

            ctx.redis
                .lock()
                .await
                .hset::<_, _, _, ()>(
                    keys::session::metadata(&ctx.external_session_id),
                    key,
                    value,
                )
                .await
                .ok();
        }
    }
}

#[derive(Deserialize)]
struct CookieRequest {
    cookie: Cookie,
}

#[derive(Deserialize)]
struct Cookie {
    name: String,
    value: String,
}
