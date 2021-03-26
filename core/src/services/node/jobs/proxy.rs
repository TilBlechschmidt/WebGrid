use super::super::Context;
use crate::libraries::lifecycle::HeartStone;
use crate::libraries::scheduling::{Job, TaskManager};
use anyhow::Result;
use async_trait::async_trait;
use hyper::{
    body,
    client::HttpConnector,
    service::{make_service_fn, service_fn},
    Body, Client as HttpClient, Error as HyperError, Method, Request, Response, Server, Version,
};
use lazy_static::lazy_static;
use log::{info, trace, warn};
use regex::Regex;
use std::net::SocketAddr;

lazy_static! {
    static ref SESSION_RE: Regex = Regex::new(r"/session/(?P<sid>[^/]*)(?:/(?P<op>.+))?").unwrap();
}

#[derive(Clone)]
pub struct ProxyJob {
    client: HttpClient<HttpConnector>,
    internal_session_id: String,
    heart_stone: HeartStone,
    port: u16,
}

#[async_trait]
impl Job for ProxyJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let internal_session_id = self.internal_session_id.clone();
        let client = self.client.clone();
        let heart_stone = self.heart_stone.clone();
        let driver_port = manager.context.options.driver_port;

        let make_svc = make_service_fn(|_conn| {
            let client = client.clone();
            let external_session_id = manager.context.id.clone();
            let internal_session_id = internal_session_id.clone();
            let heart_stone = heart_stone.clone();

            async move {
                Ok::<_, HyperError>(service_fn(move |req| {
                    ProxyJob::handle(
                        req,
                        client.clone(),
                        internal_session_id.clone(),
                        external_session_id.clone(),
                        heart_stone.clone(),
                        driver_port,
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
        external_session_id: String,
        heart_stone: &mut HeartStone,
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
            method == Method::DELETE && path == format!("/session/{}", external_session_id);
        let is_window_delete_request =
            method == Method::DELETE && path == format!("/session/{}/window", external_session_id);

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
            heart_stone
                .kill("Session closed by downstream".to_string())
                .await;
        }

        Ok(Response::from_parts(parts, Body::from(body)))
    }

    async fn handle(
        mut req: Request<Body>,
        client: HttpClient<HttpConnector, Body>,
        internal_session_id: String,
        external_session_id: String,
        mut heart_stone: HeartStone,
        driver_port: u16,
    ) -> Result<Response<Body>, HyperError> {
        // Reset the lifetime
        heart_stone.reset_lifetime().await;

        // Translate the request path
        let req_path = req
            .uri()
            .path_and_query()
            .map(|x| x.as_str().to_owned())
            .unwrap_or_else(|| "".to_string());
        let path = match SESSION_RE.captures(&req_path) {
            Some(captures) => {
                let session_id = &captures["sid"];

                if session_id == external_session_id {
                    match captures.name("op") {
                        Some(operation) => {
                            format!("/session/{}/{}", internal_session_id, operation.as_str())
                        }
                        None => format!("/session/{}", internal_session_id),
                    }
                } else {
                    // TODO Treat this as an unauthorized request and return a 421
                    req_path.to_string()
                }
            }
            None => req_path.to_string(),
        };

        // Overwrite the original path with the translated one and downgrade to HTTP/1.1
        let driver_addr: SocketAddr = ([127, 0, 0, 1], driver_port).into();
        let req_method = req.method().clone();
        let uri_string = format!("http://{}{}", driver_addr, path);
        let uri = uri_string.parse().unwrap();
        *req.uri_mut() = uri;
        *req.version_mut() = Version::HTTP_11;

        // Split the request body apart and read it for logging
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

        // Rebuild the request and create a request object
        let rebuild_req = Request::from_parts(req_parts, Body::from(req_body));
        let proxy_request = client.request(rebuild_req);

        // Dispatch the request
        match proxy_request.await {
            Ok(driver_response) => {
                ProxyJob::handle_driver_response(
                    driver_response,
                    req_path,
                    req_method,
                    external_session_id,
                    &mut heart_stone,
                )
                .await
            }
            Err(driver_response) => {
                warn!("Upstream error {}", driver_response);
                Err(driver_response)
            }
        }
    }
}
