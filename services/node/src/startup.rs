use hyper::{body, Body, Client as HttpClient, Method, Request};
use log::{debug, info};
use redis::AsyncCommands;
use std::sync::Arc;

use shared::logging::LogCode;

use crate::context::Context;
use crate::structs::{NodeError, SessionCreateResponse};

pub async fn create_local_session(ctx: Arc<Context>) -> Result<String, NodeError> {
    let mut con = ctx.con.clone();

    let capabilities_key = format!("session:{}:capabilities", ctx.config.session_id);
    let requested_capabilities: String = con
        .hget(capabilities_key, "requested")
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;
    let body_string = format!("{{\"capabilities\": {} }}", requested_capabilities);

    info!("Creating local session");
    debug!(
        "Session creation payload: {}",
        body_string.replace("\n", "")
    );

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

pub async fn resize_window(ctx: Arc<Context>, session_id: &str) -> Result<(), NodeError> {
    let path = format!("/session/{}/window/rect", session_id);
    let body_string = "{\"x\": 0, \"y\": 0, \"width\": 1920, \"height\": 1080}";
    let client = HttpClient::new();
    let req = Request::builder()
        .method(Method::POST)
        .uri(ctx.get_driver_url(&path))
        .header("Content-Type", "application/json")
        .body(Body::from(body_string))
        .map_err(|_| NodeError::LocalSessionCreationError)?;

    client
        .request(req)
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;

    Ok(())
}
