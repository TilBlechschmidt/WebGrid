use super::super::Context;
use crate::libraries::{
    resources::{ResourceManager, ResourceManagerProvider},
    tracing::StringPropagator,
};
use crate::with_redis_resource;
use crate::{libraries::helpers::keys, services::orchestrator::core::tasks::provision_session};
use anyhow::Result;
use async_trait::async_trait;
use jatsl::{Job, JobScheduler, TaskManager};
use opentelemetry::{trace::TraceContextExt, Context as TelemetryContext};
use redis::{AsyncCommands, RedisResult};

#[derive(Clone)]
pub struct ProcessorJob {}

#[async_trait]
impl Job for ProcessorJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_redis_resource!(manager);
        let orchestrator_id = manager.context.id.clone();

        manager.ready().await;

        loop {
            // While loop first to process leftover tasks from prior instance
            while let Ok(session_id) = con
                .lindex::<_, String>(keys::orchestrator::pending(&orchestrator_id), -1)
                .await
            {
                // Retrieve telemetry context
                let raw_telemetry_context: String = con
                    .hget(keys::session::telemetry::creation(&session_id), "context")
                    .await
                    .unwrap_or_default();

                let telemetry_context = TelemetryContext::current_with_span(
                    StringPropagator::deserialize(&raw_telemetry_context, "Provision node"),
                );

                // Build a provisioning context
                let provisioning_context = manager
                    .context
                    .clone()
                    .into_provisioning_context(session_id, telemetry_context);

                // Provision and observe asynchronously
                JobScheduler::spawn_task(&provision_session, provisioning_context);

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
}
