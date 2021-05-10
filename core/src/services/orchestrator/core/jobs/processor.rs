use super::super::Context;
use crate::libraries::lifecycle::logging::{LogCode, Logger};
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::{helpers::keys, resources::RedisResource};
use crate::with_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use chrono::offset::Utc;
use jatsl::{Job, TaskManager};
use log::{debug, info};
use redis::{aio::Connection, AsyncCommands, RedisResult};

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
                .lindex::<_, String>(keys::orchestrator::pending(&orchestrator_id), -1)
                .await
            {
                // Run the scheduling process
                match ProcessorJob::schedule_session(session_id.clone(), &mut con, &manager.context)
                    .await
                {
                    Ok(node_info) => {
                        debug!("Provisioned node {} {:?}", session_id, node_info);
                        logger.log(&session_id, LogCode::Sched, None).await.ok();
                    }
                    Err(e) => {
                        debug!("Failed to provision node {} {:?}", session_id, e);
                        logger.log(&session_id, LogCode::StartFail, None).await.ok();
                        manager
                            .context
                            .provisioner
                            .terminate_node(&session_id)
                            .await;
                    }
                }

                // Remove the item from the list of pending items
                con.rpop::<_, ()>(keys::orchestrator::pending(&orchestrator_id))
                    .await?;
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

    async fn schedule_session(
        session_id: String,
        con: &mut RedisResource<Connection>,
        context: &Context,
    ) -> Result<()> {
        let orchestrator_id = context.id.clone();
        info!("Starting job {}", session_id);

        // TODO Look if the job is too old

        // TODO Proper error handling, remove unwrap
        let raw_capabilities_request: String = con
            .hget(format!("session:{}:capabilities", session_id), "requested")
            .await
            .unwrap();
        let capabilities_request = serde_json::from_str(&raw_capabilities_request).unwrap();

        // TODO Add possible failure path to provisioner
        let info_future = context
            .provisioner
            .provision_node(&session_id, capabilities_request);
        let node_info = info_future.await?;

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
