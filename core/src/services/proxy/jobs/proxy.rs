use super::super::Context;
use crate::libraries::{
    metrics::MetricsEntry,
    net::discovery::{ServiceDescriptor, ServiceDiscovery},
    tracing::{self, constants::service, global_tracer},
};
use anyhow::Result;
use async_trait::async_trait;
use hyper::{
    body::HttpBody,
    service::{make_service_fn, service_fn},
};
use hyper::{client::HttpConnector, Body, Client, Method, Request, Response, Server, StatusCode};
use jatsl::{Job, TaskManager};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use opentelemetry::{
    global,
    trace::{FutureExt, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
use opentelemetry_http::HeaderInjector;
use opentelemetry_semantic_conventions as semcov;
use regex::Regex;
use std::str::FromStr;
use std::{convert::Infallible, time::Duration};
use uuid::Uuid;

static NOTFOUND: &[u8] = b"Not Found";
static NOGATEWAY: &[u8] = b"No upstream available to handle the request";

lazy_static! {
    static ref REGEX_SESSION_PATH: Regex = Regex::new(r"/session/(?P<sid>[^/]*)").unwrap();
    static ref REGEX_STORAGE_PATH: Regex = Regex::new(r"/storage/(?P<sid>[^/]*)").unwrap();
}

#[derive(Clone)]
pub struct ProxyJob {
    discovery: ServiceDiscovery,
    client: Client<HttpConnector>,
    port: u16,
    max_discovery_retries: u8,
}

#[async_trait]
impl Job for ProxyJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let make_svc = make_service_fn(|_conn| {
            let p = self.clone();
            let ctx = manager.context.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let p = p.clone();
                    let ctx = ctx.clone();
                    let tracer = tracing::global_tracer();

                    tracer.in_span(
                        "Serve request",
                        |cx| async move { p.handle(ctx, cx, req).await },
                    )
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
    pub fn new(port: u16, discovery: ServiceDiscovery) -> Self {
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .http2_only(true)
            .build_http();

        Self {
            client,
            port,
            discovery,
            max_discovery_retries: 3,
        }
    }

    async fn forward(&self, mut req: Request<Body>, upstream: String) -> Result<Response<Body>> {
        let telemetry_context =
            TelemetryContext::current_with_span(global_tracer().start("Forward request"));

        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(
                &telemetry_context,
                &mut HeaderInjector(&mut req.headers_mut()),
            )
        });

        let span = telemetry_context.span();
        span.set_attribute(tracing::constants::trace::NET_UPSTREAM_NAME.string(upstream.clone()));

        let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

        debug!("{} {} -> {}", req.method(), path, upstream);

        *req.uri_mut() = format!("http://{}{}", upstream, path).parse().unwrap();

        span.add_event("Sending request".to_string(), vec![]);
        let result = self.client.request(req).await;
        span.add_event("Received response".to_string(), vec![]);

        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Failed to fulfill request to '{}': {}", upstream, e);
                let error_message = format!("Unable to forward request to {}: {}", upstream, e);

                span.set_status(opentelemetry::trace::StatusCode::Error, error_message);

                Err(e.into())
            }
        }
    }

    #[allow(clippy::never_loop)]
    async fn forward_to_service(
        &self,
        service: ServiceDescriptor,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        let span = global_tracer().start("Forward to service");
        let telemetry_context = TelemetryContext::current_with_span(span);

        let service_string = service.to_string();
        let mut discoverer = self
            .discovery
            .start_discovery(service, self.max_discovery_retries);

        loop {
            match discoverer
                .discover()
                .with_context(telemetry_context.clone())
                .await
            {
                Ok(endpoint) => {
                    debug!("Attempting connection to {}", endpoint);
                    match self
                        .forward(req, endpoint.clone())
                        .with_context(telemetry_context.clone())
                        .await
                    {
                        Ok(res) => return Ok(res),
                        Err(_) => {
                            warn!("Flagging endpoint {} as unreachable", endpoint);
                            discoverer.flag_stale(&endpoint).await;
                        }
                    }

                    // TODO We kinda don't want to break after the first retry.
                    //      However, req is currently not duplicatable ...
                    break;
                }
                Err(e) => {
                    error!(
                        "Failed to fulfill request to service {}: {}",
                        service_string, e
                    );
                    break;
                }
            }
        }

        Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(NOGATEWAY.into())
            .unwrap())
    }

    async fn handle(
        &self,
        context: Context,
        telemetry_context: TelemetryContext,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        let req_method = req.method().clone();
        let req_size = req.body().size_hint().lower();
        context
            .metrics
            .submit(MetricsEntry::IncomingTraffic(req_size))
            .ok();

        let path = req
            .uri()
            .path_and_query()
            .map(|x| x.to_string())
            .unwrap_or_else(|| "".to_string());

        let span = telemetry_context.span();
        span.set_attribute(
            semcov::trace::HTTP_FLAVOR.string(format!("{:?}", req.version()).replace("HTTP/", "")),
        );
        span.set_attribute(semcov::trace::HTTP_METHOD.string(req_method.to_string()));
        span.set_attribute(semcov::trace::HTTP_REQUEST_CONTENT_LENGTH.string(req_size.to_string()));
        span.set_attribute(semcov::trace::HTTP_TARGET.string(path.clone()));

        let not_found_response = Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap());

        let result = if req.method() == Method::POST && path == "/session" {
            span.set_attribute(semcov::trace::HTTP_ROUTE.string("/session"));
            span.set_attribute(semcov::trace::PEER_SERVICE.string(service::MANAGER));
            span.update_name("/session".to_string());

            self.forward_to_service(ServiceDescriptor::Manager, req)
                .with_context(telemetry_context.clone())
                .await
        } else if path.starts_with("/storage/") {
            match REGEX_STORAGE_PATH.captures(&path) {
                Some(caps) => {
                    span.set_attribute(semcov::trace::HTTP_ROUTE.string("/storage/:storage_id/*"));
                    span.set_attribute(semcov::trace::PEER_SERVICE.string(service::STORAGE));
                    span.update_name("/storage/:storage_id/*".to_string());

                    match Uuid::from_str(&caps["sid"]) {
                        Ok(id) => {
                            self.forward_to_service(ServiceDescriptor::Storage(id), req)
                                .with_context(telemetry_context.clone())
                                .await
                        }
                        _ => not_found_response,
                    }
                }
                None => {
                    debug!("{} {} -> NOT FOUND", req.method(), path);
                    not_found_response
                }
            }
        } else if path.starts_with("/session/") {
            match REGEX_SESSION_PATH.captures(&path) {
                Some(caps) => {
                    span.set_attribute(semcov::trace::HTTP_ROUTE.string("/session/:session_id/*"));
                    span.set_attribute(semcov::trace::PEER_SERVICE.string(service::NODE));
                    span.update_name("/session/:session_id/*".to_string());

                    match Uuid::from_str(&caps["sid"]) {
                        Ok(id) => {
                            self.forward_to_service(ServiceDescriptor::Node(id), req)
                                .with_context(telemetry_context.clone())
                                .await
                        }
                        _ => not_found_response,
                    }
                }
                None => {
                    debug!("{} {} -> NOT FOUND", req.method(), path);
                    not_found_response
                }
            }
        } else {
            // Send all unmatched requests to the API since it serves the
            // dashboard which might cover some paths we don't know about.
            span.set_attribute(semcov::trace::HTTP_ROUTE.string("*"));
            span.set_attribute(semcov::trace::PEER_SERVICE.string(service::API));
            span.update_name("/*".to_string());

            self.forward_to_service(ServiceDescriptor::Api, req)
                .with_context(telemetry_context.clone())
                .await
        };

        if let Ok(response) = &result {
            let status_code = response.status();
            let res_size = response.body().size_hint().lower();

            span.set_attribute(
                semcov::trace::HTTP_STATUS_CODE.string(status_code.as_u16().to_string()),
            );
            span.set_attribute(
                semcov::trace::HTTP_RESPONSE_CONTENT_LENGTH.string(res_size.to_string()),
            );

            context
                .metrics
                .submit(MetricsEntry::OutgoingTraffic(res_size))
                .ok();
            context
                .metrics
                .submit(MetricsEntry::RequestProcessed(req_method, status_code))
                .ok();
        } else if let Err(e) = &result {
            error!(
                "Encountered error while serving {} request to {}: {}",
                req_method, path, e
            );

            context
                .metrics
                .submit(MetricsEntry::RequestProcessed(
                    req_method,
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
                .ok();
        }

        result
    }
}
