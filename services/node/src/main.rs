#[macro_use]
extern crate lazy_static;
use chrono::prelude::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{
    body, Body, Client as HttpClient, Error as HyperError, Method, Request, Response, Server,
};
use log::{debug, error, info, warn};
use redis::{AsyncCommands, RedisResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Error as IOError;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use shared::lifecycle::{generate_session_termination_script, wait_for, DeathReason};
use shared::logging::LogCode;
use shared::Timeout;

mod config;
mod context;
mod driver;

use crate::context::Context;

#[derive(Debug)]
enum NodeError {
    DriverStart(IOError),
    NoDriverResponse,
    LocalSessionCreationError,
}

#[derive(Serialize, Deserialize)]
struct SessionCreateResponseValue {
    #[serde(rename = "sessionId")]
    session_id: String,
    capabilities: Value,
}

#[derive(Deserialize)]
struct SessionCreateResponse {
    value: SessionCreateResponseValue,
}

async fn await_driver_startup(ctx: Arc<Context>) -> Result<(), NodeError> {
    let timeout = Timeout::DriverStartup.get(&ctx.con).await;

    info!("Awaiting driver startup");

    match wait_for(
        &ctx.get_driver_url("/status"),
        Duration::from_secs(timeout as u64),
    )
    .await
    {
        Ok(_) => {
            info!("Driver became responsive");
            Ok(())
        }
        Err(_) => {
            error!("Timeout waiting for driver startup");
            ctx.logger.log(LogCode::DTIMEOUT, None).await.ok();
            Err(NodeError::NoDriverResponse)
        }
    }
}

async fn start_driver(ctx: Arc<Context>) -> Result<(), NodeError> {
    ctx.logger.log(LogCode::DSTART, None).await.ok();

    info!("Starting local driver");

    match ctx.driver.start() {
        Ok(_) => {
            await_driver_startup(ctx.clone()).await?;
            ctx.logger.log(LogCode::DALIVE, None).await.ok();
            Ok(())
        }
        Err(e) => {
            error!("Failed to start driver {}", e);
            ctx.logger
                .log(LogCode::DFAILURE, Some(format!("{}", e)))
                .await
                .ok();
            Err(NodeError::DriverStart(e))
        }
    }
}

async fn create_local_session(
    ctx: Arc<Context>,
    requested_capabilities: String,
) -> Result<String, NodeError> {
    let mut con = ctx.con.clone();
    let body_string = format!("{{\"capabilities\": {} }}", requested_capabilities);

    info!("Creating local session");

    let client = HttpClient::new();
    let req = Request::builder()
        .method(Method::POST)
        .uri(ctx.get_driver_url("/session"))
        .header("Content-Type", "application/json")
        .body(Body::from(body_string))
        .map_err(|_| NodeError::LocalSessionCreationError)?;

    let res = client
        .request(req)
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;
    let bytes = body::to_bytes(res.into_body())
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;
    let body =
        String::from_utf8(bytes.to_vec()).map_err(|_| NodeError::LocalSessionCreationError)?;

    let response: SessionCreateResponse =
        serde_json::from_str(&body).map_err(|_| NodeError::LocalSessionCreationError)?;
    let session_id = response.value.session_id.clone();
    let capabilities = serde_json::to_string(&response.value.capabilities)
        .map_err(|_| NodeError::LocalSessionCreationError)?;

    ctx.logger.log(LogCode::LSINIT, None).await.ok();

    con.hset(
        format!("session:{}:upstream", &ctx.config.session_id),
        "driverSessionID",
        &session_id,
    )
    .await
    .map_err(|_| NodeError::LocalSessionCreationError)?;

    con.hset(
        format!("session:{}:capabilities", &ctx.config.session_id),
        "actual",
        capabilities,
    )
    .await
    .map_err(|_| NodeError::LocalSessionCreationError)?;

    info!("Created local session {}", session_id);

    Ok(session_id)
}

async fn serve_proxy(ctx: Arc<Context>, internal_session_id: String) {
    let in_addr = ([0, 0, 0, 0], 3030).into();
    let out_addr: SocketAddr = ctx.driver_addr;
    let client_main = HttpClient::new();

    info!("WebDriver proxy serving {:?} -> {:?}", in_addr, out_addr);

    let make_service = make_service_fn(move |_| {
        let ctx = ctx.clone();
        let client = client_main.clone();
        let external_session_id = ctx.config.session_id.clone();
        let internal_session_id = internal_session_id.clone();

        lazy_static! {
            static ref SESSION_RE: Regex =
                Regex::new(r"/session/(?P<sid>[^/]*)(?:/(?P<op>.+))?").unwrap();
        }

        async move {
            Ok::<_, HyperError>(service_fn(move |mut req| {
                let ctx = ctx.clone();

                let request_path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");

                let is_session_delete_request = req.method() == Method::DELETE
                    && request_path == format!("/session/{}", external_session_id);
                let is_window_delete_request = req.method() == Method::DELETE
                    && request_path == format!("/session/{}/window", external_session_id);

                let path = match SESSION_RE.captures(&request_path) {
                    Some(captures) => {
                        let session_id = &captures["sid"];

                        if session_id == external_session_id {
                            match captures.name("op") {
                                Some(operation) => format!(
                                    "/session/{}/{}",
                                    internal_session_id,
                                    operation.as_str()
                                ),
                                None => format!("/session/{}", internal_session_id),
                            }
                        } else {
                            request_path.to_string()
                        }
                    }
                    None => request_path.to_string(),
                };

                let uri = format!("http://{}{}", out_addr, path).parse().unwrap();
                debug!("{} {} -> {}", req.method(), path, uri);
                *req.uri_mut() = uri;

                let proxy_request = client.request(req);

                async move {
                    let mut con = ctx.con.clone();
                    let _: RedisResult<()> = con
                        .hset(
                            format!("session:{}:downstream", ctx.config.session_id),
                            "lastSeen",
                            Utc::now().to_rfc3339(),
                        )
                        .await;
                    let _: RedisResult<()> = con
                        .hincr(
                            format!("session:{}:downstream", ctx.config.session_id),
                            "fulfilledRequests",
                            1,
                        )
                        .await;

                    ctx.heart.reset_lifetime();

                    match proxy_request.await {
                        Err(driver_response) => {
                            warn!("Upstream error {}", driver_response);
                            Err(driver_response)
                        }
                        Ok(driver_response) => {
                            debug!("Upstream {} code {}", path, driver_response.status());
                            let (parts, body) = driver_response.into_parts();

                            let body = match body::to_bytes(body).await {
                                Ok(bytes) => String::from_utf8(bytes.to_vec())
                                    .unwrap_or_else(|_| "".to_string()),
                                Err(_) => "".to_string(),
                            };

                            let session_closed = if is_window_delete_request {
                                lazy_static! {
                                    static ref EMPTY_VALUE_RE: Regex =
                                        Regex::new(r#""value": ?\[\]"#).unwrap();
                                }

                                EMPTY_VALUE_RE.is_match(&body)
                            } else {
                                is_session_delete_request
                            };

                            if session_closed {
                                warn!("Session closed by downstream");
                                ctx.heart.kill();
                            }

                            Ok(Response::from_parts(parts, Body::from(body)))
                        }
                    }
                }
            }))
        }
    });

    let server = Server::bind(&in_addr).serve(make_service);
    tokio::spawn(server);
}

fn call_on_create_script(ctx: Arc<Context>) {
    match &ctx.config.on_session_create {
        Some(script) => {
            info!("Calling on_create_script {}", script);
            let parts: Vec<String> = script.split_whitespace().map(|s| s.to_string()).collect();
            let process = Command::new(parts[0].clone()).args(&parts[1..]).spawn();
            if let Err(e) = process {
                error!("Failed to execute on_create_script {:?}", e);
            }
        }
        None => {}
    }
}

async fn node_startup(ctx: Arc<Context>) -> Result<(), NodeError> {
    start_driver(ctx.clone()).await?;
    let session_id = create_local_session(ctx.clone(), "{}".to_string()).await?;
    serve_proxy(ctx.clone(), session_id).await;
    call_on_create_script(ctx);

    Ok(())
}

async fn terminate_session(ctx: Arc<Context>) {
    ctx.driver.stop();

    let mut con = ctx.con.clone();
    let _: Option<()> = generate_session_termination_script(false)
        .arg(ctx.config.session_id.clone())
        .arg(Utc::now().to_rfc3339())
        .invoke_async(&mut con)
        .await
        .ok();
}

#[tokio::main]
async fn main() {
    let ctx = Arc::new(Context::new().await);
    ctx.heart
        .add_beat(
            format!("session:{}:heartbeat.node", ctx.config.session_id),
            60,
            120,
        )
        .await;

    ctx.logger.log(LogCode::BOOT, None).await.ok();

    let ctx_startup = ctx.clone();
    tokio::spawn(async {
        let heart = ctx_startup.heart.clone();
        if let Err(e) = node_startup(ctx_startup).await {
            error!("Startup routine failed {:?}", e);
            heart.kill();
        }
    });

    // The heart will keep beating until either the session is closed, a timeout occurs or the signal handler triggers.
    match ctx.heart.beat(true).await {
        DeathReason::LifetimeExceeded => {
            info!("Lifetime was exceeded");
            ctx.logger.log(LogCode::STIMEOUT, None).await.ok();
        }
        DeathReason::Killed => {
            ctx.logger.log(LogCode::CLOSED, None).await.ok();
        }
    }

    terminate_session(ctx.clone()).await;
    ctx.logger.log(LogCode::HALT, None).await.ok();
}
