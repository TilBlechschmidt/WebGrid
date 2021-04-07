use super::super::{
    structs::{NodeError, SessionCreateResponse},
    Context,
};
use crate::libraries::helpers::keys;
use crate::libraries::lifecycle::logging::{LogCode, SessionLogger};
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::with_redis_resource;
use hyper::{body, Body, Client as HttpClient, Method, Request};
use jatsl::TaskManager;
use log::{debug, error, info};
use redis::{aio::ConnectionLike, AsyncCommands};
use std::{net::SocketAddr, process::Command};

pub async fn initialize_session(manager: TaskManager<Context>) -> Result<String, NodeError> {
    let external_session_id: String = manager.context.id.clone();
    let driver_port = manager.context.options.driver_port;
    let on_session_create = manager.context.options.on_session_create.clone();

    let log_con = with_redis_resource!(manager);
    let mut logger = SessionLogger::new(log_con, "node".to_string(), external_session_id.clone());

    let internal_session_id = subtasks::create_local_session(manager, &mut logger).await?;

    subtasks::resize_window(&internal_session_id, driver_port).await?;

    if let Some(ref script) = on_session_create {
        subtasks::call_on_create_script(&script);
    }

    Ok(internal_session_id)
}

mod subtasks {
    use super::*;

    pub async fn create_local_session<C: ConnectionLike + AsyncCommands>(
        manager: TaskManager<Context>,
        logger: &mut SessionLogger<C>,
    ) -> Result<String, NodeError> {
        let external_session_id: String = manager.context.id.clone();
        let driver_port = manager.context.options.driver_port;
        let mut con = with_redis_resource!(manager);

        // Read the requested capabilities from the database and construct a request body
        let requested_capabilities: String = con
            .hget(
                keys::session::capabilities(&external_session_id),
                "requested",
            )
            .await
            .map_err(|_| NodeError::LocalSessionCreationError)?;
        let body_string = format!("{{\"capabilities\": {} }}", requested_capabilities);

        info!("Creating local session");
        debug!(
            "Session creation payload: {}",
            body_string.replace("\n", "")
        );

        // Construct a request to the local driver
        let client = HttpClient::new();
        let socket_addr: SocketAddr = ([127, 0, 0, 1], driver_port).into();
        let url = format!("http://{}/session", socket_addr);
        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Body::from(body_string))
            .map_err(|_| NodeError::LocalSessionCreationError)?;

        // Send the request and deconstruct the response body
        let res = client
            .request(req)
            .await
            .map_err(|_| NodeError::LocalSessionCreationError)?;
        let bytes = body::to_bytes(res.into_body())
            .await
            .map_err(|_| NodeError::LocalSessionCreationError)?;
        let body =
            String::from_utf8(bytes.to_vec()).map_err(|_| NodeError::LocalSessionCreationError)?;

        debug!("Session creation response: {}", body.replace("\n", ""));

        let response: SessionCreateResponse =
            serde_json::from_str(&body).map_err(|_| NodeError::LocalSessionCreationError)?;
        let internal_session_id: String = response.value.session_id.clone();
        let capabilities = serde_json::to_string(&response.value.capabilities)
            .map_err(|_| NodeError::LocalSessionCreationError)?;

        logger.log(LogCode::LsInit, None).await.ok();

        // Upload the resulting internal session ID and actual capabilities to the database
        con.hset(
            keys::session::upstream(&external_session_id),
            "driverSessionID",
            &external_session_id,
        )
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;

        con.hset(
            keys::session::capabilities(&external_session_id),
            "actual",
            capabilities,
        )
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;

        info!("Created local session {}", internal_session_id);

        Ok(internal_session_id)
    }

    pub async fn resize_window(session_id: &str, driver_port: u16) -> Result<(), NodeError> {
        let socket_addr: SocketAddr = ([127, 0, 0, 1], driver_port).into();
        let url = format!("http://{}/session/{}/window/rect", socket_addr, session_id);
        let body_string = "{\"x\": 0, \"y\": 0, \"width\": 1920, \"height\": 1080}";
        let client = HttpClient::new();
        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .body(Body::from(body_string))
            .map_err(|_| NodeError::LocalSessionCreationError)?;

        client
            .request(req)
            .await
            .map_err(|_| NodeError::LocalSessionCreationError)?;

        Ok(())
    }

    pub fn call_on_create_script(script: &str) {
        info!("Calling on_create_script {}", script);

        let parts: Vec<String> = script.split_whitespace().map(|s| s.to_string()).collect();
        let process = Command::new(parts[0].clone()).args(&parts[1..]).spawn();

        if let Err(e) = process {
            error!("Failed to execute on_create_script {:?}", e);
        }
    }
}
