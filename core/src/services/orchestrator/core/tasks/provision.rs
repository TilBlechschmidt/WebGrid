use super::super::ProvisioningContext;
use crate::libraries::resources::RedisResource;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::{libraries::helpers::keys, with_shared_redis_resource};
use anyhow::{anyhow, Result};
use chrono::Utc;
use jatsl::TaskManager;
use log::{debug, error, info};
use opentelemetry::{
    trace::{FutureExt, StatusCode, TraceContextExt},
    Context as TelemetryContext,
};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use std::str::FromStr;

pub async fn provision_session(manager: TaskManager<ProvisioningContext>) -> Result<()> {
    let mut con = with_shared_redis_resource!(manager);
    let session_id = manager.context.session_id.clone();

    match subtasks::provision_session(&mut con, &manager.context).await {
        Ok(node_info) => {
            debug!("Provisioned node {} {:?}", session_id, node_info);

            redis::pipe()
                .atomic()
                .cmd("HSETNX")
                .arg(keys::session::status(&session_id))
                .arg("pendingAt")
                .arg(Utc::now().to_rfc3339())
                .cmd("RPUSH")
                .arg(keys::session::orchestrator(&session_id))
                .arg(&manager.context.id)
                .query_async(&mut con)
                .await?;
        }
        Err(e) => {
            error!("Failed to provision node {} {:?}", session_id, e);
            manager
                .context
                .telemetry_context
                .span()
                .set_status(StatusCode::Error, e.to_string());
            manager
                .context
                .provisioner
                .terminate_node(&session_id)
                .await;
        }
    };

    Ok(())
}

mod subtasks {
    use super::*;
    use crate::libraries::{
        helpers::{wait_for, wait_for_key, Timeout},
        net::discovery::ServiceDescriptor,
        tracing::global_tracer,
    };
    use futures::TryFutureExt;
    use opentelemetry::trace::{Span, Tracer};
    use std::time::{Duration, Instant};
    use uuid::Uuid;

    pub async fn provision_session(
        con: &mut RedisResource<MultiplexedConnection>,
        context: &ProvisioningContext,
    ) -> Result<()> {
        let session_id = context.session_id.clone();
        let span = context.telemetry_context.span();
        info!("Provisioning {}", session_id);

        // TODO Look if the job is too old

        let raw_capabilities_request: String = con
            .hget(format!("session:{}:capabilities", session_id), "requested")
            .await?;
        let capabilities_request = serde_json::from_str(&raw_capabilities_request)?;

        span.add_event("Delegating to provisioner".to_string(), vec![]);

        context
            .provisioner
            .provision_node(&session_id, capabilities_request)
            .with_context(context.telemetry_context.clone())
            .await?;

        await_startup(con, context)
            .with_context(context.telemetry_context.clone())
            .await?;

        Ok(())
    }

    async fn await_startup(
        con: &mut RedisResource<MultiplexedConnection>,
        context: &ProvisioningContext,
    ) -> Result<()> {
        let tracer = global_tracer();
        let mut span = tracer.start("Await session startup");
        let timeout = Timeout::NodeStartup.get(con).await as u64;
        let timeout_duration = Duration::from_secs(timeout);
        let healthcheck_start = Instant::now();

        // Wait for the node to send heart-beats
        wait_for_key(
            &keys::session::heartbeat::node(&context.session_id),
            timeout_duration,
            con,
        )
        .map_err(|_| {
            span.set_status(
                StatusCode::Error,
                "Timed out waiting for heartbeat".to_string(),
            );

            anyhow!("Timed out waiting for heartbeat")
        })
        .await?;

        span.add_event("Waiting for status endpoint".to_string(), vec![]);

        // Wait for the HTTP endpoint to be reachable
        let session_id = Uuid::from_str(&context.session_id)?;
        let telemetry_context = TelemetryContext::current_with_span(span);
        let remaining_duration =
            timeout_duration - Instant::now().duration_since(healthcheck_start);

        // Discover the endpoint to watch
        let mut discover = context
            .discovery
            .start_discovery(ServiceDescriptor::Node(session_id), 10);
        let endpoint = discover.discover().await?;
        let url = format!("http://{}/status", endpoint);

        // Query the HTTP status probe
        wait_for(&url, remaining_duration)
            .with_context(telemetry_context.clone())
            .map_err(|_| {
                telemetry_context.span().set_status(
                    StatusCode::Error,
                    "Timed out waiting for status endpoint".to_string(),
                );

                anyhow!("Timed out waiting for heartbeat")
            })
            .await?;

        Ok(())
    }
}
