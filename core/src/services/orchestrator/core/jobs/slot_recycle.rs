use super::super::Context;
use crate::libraries::helpers::keys;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::scheduling::{Job, TaskManager};
use crate::with_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use redis::{AsyncCommands, RedisResult};

#[derive(Clone)]
pub struct SlotRecycleJob {}

#[async_trait]
impl Job for SlotRecycleJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_redis_resource!(manager);
        let orchestrator_id = manager.context.id.clone();

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
