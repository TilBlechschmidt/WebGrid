use super::super::{routing_info::RoutingInfo, Context};
use crate::libraries::{
    metrics::MetricsEntry,
    scheduling::{Job, TaskManager},
};
use anyhow::Result;
use async_trait::async_trait;
use hyper::{
    body::HttpBody,
    service::{make_service_fn, service_fn},
};
use hyper::{client::HttpConnector, Body, Client, Method, Request, Response, Server, StatusCode};
use lazy_static::lazy_static;
use log::{debug, error, info};
use regex::Regex;
use std::convert::Infallible;

static NOTFOUND: &[u8] = b"Not Found";
static NOGATEWAY: &[u8] = b"No upstream available to handle the request";

lazy_static! {
    static ref REGEX_SESSION_PATH: Regex = Regex::new(r"/session/(?P<sid>[^/]*)").unwrap();
    static ref REGEX_STORAGE_PATH: Regex = Regex::new(r"/storage/(?P<sid>[^/]*)").unwrap();
}

#[derive(Clone)]
pub struct ProxyJob {
    client: Client<HttpConnector>,
    port: u16,
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
                    async move { p.handle(ctx, req).await }
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
    pub fn new(port: u16) -> Self {
        Self {
            client: Client::new(),
            port,
        }
    }

    async fn forward(&self, mut req: Request<Body>, upstream: String) -> Result<Response<Body>> {
        let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

        debug!("{} {} -> {}", req.method(), path, upstream);

        *req.uri_mut() = format!("http://{}{}", upstream, path).parse().unwrap();

        let result = self.client.request(req).await;

        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Failed to fulfill request to '{}': {}", upstream, e);
                let error_message = format!("Unable to forward request to {}: {}", upstream, e);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(error_message.into())
                    .unwrap())
            }
        }
    }

    async fn handle_session_request(
        &self,
        req: Request<Body>,
        session_id: &str,
        info: &RoutingInfo,
    ) -> Result<Response<Body>> {
        match info.get_session_upstream(session_id).await {
            Some(upstream) => self.forward(req, upstream).await,
            None => {
                let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                debug!("{} {} -> BAD GATEWAY (session request)", req.method(), path);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(NOGATEWAY.into())
                    .unwrap())
            }
        }
    }

    async fn handle_manager_request(
        &self,
        req: Request<Body>,
        info: &RoutingInfo,
    ) -> Result<Response<Body>> {
        match info.get_manager_upstream().await {
            Some(upstream) => self.forward(req, upstream).await,
            None => {
                let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                debug!("{} {} -> BAD GATEWAY (manager request)", req.method(), path);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(NOGATEWAY.into())
                    .unwrap())
            }
        }
    }

    async fn handle_api_request(
        &self,
        req: Request<Body>,
        info: &RoutingInfo,
    ) -> Result<Response<Body>> {
        match info.get_api_upstream().await {
            Some(upstream) => self.forward(req, upstream).await,
            None => {
                let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                debug!("{} {} -> BAD GATEWAY (api request)", req.method(), path);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(NOGATEWAY.into())
                    .unwrap())
            }
        }
    }

    async fn handle_storage_request(
        &self,
        req: Request<Body>,
        storage_id: &str,
        info: &RoutingInfo,
    ) -> Result<Response<Body>> {
        match info.get_storage_upstream(storage_id).await {
            Some(upstream) => self.forward(req, upstream).await,
            None => {
                let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
                debug!("{} {} -> BAD GATEWAY (storage request)", req.method(), path);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(NOGATEWAY.into())
                    .unwrap())
            }
        }
    }

    async fn handle(&self, context: Context, req: Request<Body>) -> Result<Response<Body>> {
        let info = context.routing_info;
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

        let result = if req.method() == Method::POST && path == "/session" {
            self.handle_manager_request(req, &info).await
        } else if path.starts_with("/api") || path.starts_with("/embed") {
            self.handle_api_request(req, &info).await
        } else if path.starts_with("/storage") {
            match REGEX_STORAGE_PATH.captures(&path) {
                Some(caps) => self.handle_storage_request(req, &caps["sid"], &info).await,
                None => {
                    debug!("{} {} -> NOT FOUND", req.method(), path);

                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(NOTFOUND.into())
                        .unwrap())
                }
            }
        } else {
            match REGEX_SESSION_PATH.captures(&path) {
                Some(caps) => self.handle_session_request(req, &caps["sid"], &info).await,
                None => {
                    debug!("{} {} -> NOT FOUND", req.method(), path);

                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(NOTFOUND.into())
                        .unwrap())
                }
            }
        };

        if let Ok(response) = &result {
            let status_code = response.status();
            let res_size = response.body().size_hint().lower();

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
