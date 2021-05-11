use super::super::ProvisioningContext;
use crate::libraries::resources::RedisResource;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::{libraries::helpers::keys, with_shared_redis_resource};
use anyhow::Result;
use chrono::Utc;
use jatsl::TaskManager;
use log::{debug, error, info};
use opentelemetry::trace::{FutureExt, StatusCode, TraceContextExt};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

pub async fn provision_session(manager: TaskManager<ProvisioningContext>) -> Result<()> {
    let mut con = with_shared_redis_resource!(manager);
    let session_id = manager.context.session_id.clone();

    match subtasks::schedule_session(&mut con, &manager.context).await {
        Ok(node_info) => {
            debug!("Provisioned node {} {:?}", session_id, node_info);
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

    pub async fn schedule_session(
        con: &mut RedisResource<MultiplexedConnection>,
        context: &ProvisioningContext,
    ) -> Result<()> {
        let orchestrator_id = context.id.clone();
        let session_id = context.session_id.clone();
        let span = context.telemetry_context.span();
        info!("Starting job {}", session_id);

        // TODO Look if the job is too old

        let raw_capabilities_request: String = con
            .hget(format!("session:{}:capabilities", session_id), "requested")
            .await?;
        let capabilities_request = serde_json::from_str(&raw_capabilities_request)?;

        span.add_event("Delegating to provisioner".to_string(), vec![]);

        // TODO Add possible failure path to provisioner
        let info_future = context
            .provisioner
            .provision_node(&session_id, capabilities_request)
            .with_context(context.telemetry_context.clone());
        let node_info = info_future.await?;

        span.add_event("Persist node info".to_string(), vec![]);

        // TODO Add node info to span

        redis::pipe()
            .atomic()
            .cmd("HSETNX")
            .arg(keys::session::status(&session_id))
            .arg("pendingAt")
            .arg(Utc::now().to_rfc3339())
            .cmd("RPUSH")
            .arg(keys::session::orchestrator(&session_id))
            .arg(&orchestrator_id)
            .cmd("HMSET")
            .arg(keys::session::upstream(&session_id))
            .arg("host")
            .arg(&node_info.host)
            .arg("port")
            .arg(&node_info.port)
            .query_async(con)
            .await?;

        Ok(())
    }
}
