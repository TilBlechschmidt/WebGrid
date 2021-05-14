use super::super::structs::{NodeError, SessionCreateResponse};
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::tracing::global_tracer;
use crate::with_redis_resource;
use crate::{libraries::helpers::keys, services::node::context::StartupContext};
use hyper::{body, Body, Client as HttpClient, Method, Request};
use jatsl::TaskManager;
use log::{debug, error, info};
use opentelemetry::{
    trace::{FutureExt, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
use redis::AsyncCommands;
use std::{net::SocketAddr, process::Command};

pub async fn initialize_session(manager: TaskManager<StartupContext>) -> Result<String, NodeError> {
    let span = global_tracer().start_with_context(
        "Configure local session",
        manager.context.telemetry_context.clone(),
    );
    let telemetry_context = TelemetryContext::current_with_span(span);

    let driver_port = manager.context.options.driver_port;
    let on_session_create = manager.context.options.on_session_create.clone();

    let internal_session_id = subtasks::create_local_session(manager)
        .with_context(telemetry_context.clone())
        .await?;

    subtasks::resize_window(&internal_session_id, driver_port)
        .with_context(telemetry_context.clone())
        .await?;

    if let Some(ref script) = on_session_create {
        telemetry_context
            .span()
            .add_event("Calling on_create_script".to_string(), vec![]);
        subtasks::call_on_create_script(&script);
    }

    Ok(internal_session_id)
}

mod subtasks {
    use opentelemetry::trace::Span;

    use crate::libraries::tracing::constants::trace;

    use super::*;

    pub async fn create_local_session(
        manager: TaskManager<StartupContext>,
    ) -> Result<String, NodeError> {
        let span = global_tracer().start("Create session");

        let external_session_id: String = manager.context.id.to_string();
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
        span.add_event("Sending request".to_string(), vec![]);
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
        span.add_event("Received response".to_string(), vec![]);

        let response: SessionCreateResponse =
            serde_json::from_str(&body).map_err(|_| NodeError::LocalSessionCreationError)?;
        let internal_session_id: String = response.value.session_id.clone();
        let capabilities = serde_json::to_string(&response.value.capabilities)
            .map_err(|_| NodeError::LocalSessionCreationError)?;

        // Upload the actual capabilities to the database
        con.hset(
            keys::session::capabilities(&external_session_id),
            "actual",
            capabilities,
        )
        .await
        .map_err(|_| NodeError::LocalSessionCreationError)?;

        info!("Created local session {}", internal_session_id);
        span.set_attribute(trace::SESSION_ID_INTERNAL.string(internal_session_id.clone()));

        Ok(internal_session_id)
    }

    pub async fn resize_window(session_id: &str, driver_port: u16) -> Result<(), NodeError> {
        let span = global_tracer().start("Resize window");

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

        span.add_event("Sending request".to_string(), vec![]);
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
