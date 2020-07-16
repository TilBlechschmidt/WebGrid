use super::super::Context;
use anyhow::Result;
use async_trait::async_trait;
use chrono::offset::Utc;
use helpers::keys;
use lifecycle::logging::{LogCode, Logger};
use log::{debug, info};
use redis::{AsyncCommands, RedisResult};
use resources::{with_redis_resource, ResourceManager};
use scheduling::{Job, TaskManager};

#[derive(Clone)]
pub struct ProcessorJob {}

#[async_trait]
impl Job for ProcessorJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_redis_resource!(manager);
        let log_con = with_redis_resource!(manager);
        let mut logger = Logger::new(log_con, "orchestrator".to_string());
        let orchestrator_id = manager.context.id.clone();

        manager.ready().await;

        loop {
            // While loop first to process leftover tasks from prior instance
            while let Ok(session_id) = con
                .lindex(keys::orchestrator::pending(&orchestrator_id), -1)
                .await
            {
                let session_id: String = session_id;
                info!("Starting job {}", session_id);

                // TODO Look if the job is too old

                // TODO Proper error handling, remove unwrap
                let raw_capabilities_request: String = con
                    .hget(format!("session:{}:capabilities", session_id), "requested")
                    .await
                    .unwrap();
                let capabilities_request = serde_json::from_str(&raw_capabilities_request).unwrap();

                let info_future = manager
                    .context
                    .provisioner
                    .provision_node(&session_id, capabilities_request);
                let node_info = info_future.await;

                let status_key = format!("session:{}:status", session_id);
                let orchestrator_key = format!("session:{}:orchestrator", session_id);
                let upstream_key = format!("session:{}:upstream", session_id);
                let timestamp = Utc::now().to_rfc3339();

                let result: RedisResult<()> = redis::pipe()
                    .atomic()
                    .cmd("RPOP")
                    .arg(keys::orchestrator::pending(&orchestrator_id))
                    .cmd("HSETNX")
                    .arg(status_key)
                    .arg("pendingAt")
                    .arg(timestamp)
                    .cmd("RPUSH")
                    .arg(orchestrator_key)
                    .arg(&orchestrator_id)
                    .cmd("HMSET")
                    .arg(upstream_key)
                    .arg("host")
                    .arg(&node_info.host)
                    .arg("port")
                    .arg(&node_info.port)
                    .query_async(&mut con)
                    .await;

                if result.is_err() {
                    debug!("Failed to provision node {} {:?}", session_id, result);
                    logger.log(&session_id, LogCode::STARTFAIL, None).await.ok();
                    manager
                        .context
                        .provisioner
                        .terminate_node(&session_id)
                        .await;
                } else {
                    debug!("Provisioned node {} {:?}", session_id, node_info);
                    logger.log(&session_id, LogCode::SCHED, None).await.ok();
                }
            }

            let _: RedisResult<()> = con
                .brpoplpush(
                    keys::orchestrator::backlog(&orchestrator_id),
                    keys::orchestrator::pending(&orchestrator_id),
                    0,
                )
                .await;
        }
    }
}

impl ProcessorJob {
    pub fn new() -> Self {
        Self {}
    }
}
