use crate::Context;
use anyhow::Result;
use async_trait::async_trait;
use helpers::{env, keys};
use log::info;
use redis::{AsyncCommands, RedisResult};
use resources::{with_redis_resource, ResourceManager};
use scheduling::{Job, TaskManager};

#[derive(Clone)]
pub struct SlotRecycleJob {}

#[async_trait]
impl Job for SlotRecycleJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_redis_resource!(manager);
        let orchestrator_id = (*env::service::orchestrator::ID).clone();

        let source = keys::orchestrator::slots::reclaimed(&orchestrator_id);
        let destination = keys::orchestrator::slots::available(&orchestrator_id);

        manager.ready().await;

        loop {
            let slot: RedisResult<String> = con.brpoplpush(&source, &destination, 0).await;

            // TODO Add safety check if slot actually belongs to this orchestrator, print a warning and discard it if not!

            if let Ok(slot) = slot {
                info!("Recycled slot {}", slot);
            }
        }
    }
}

impl SlotRecycleJob {
    pub fn new() -> Self {
        Self {}
    }
}
