use super::super::Context;
use anyhow::Result;
use async_trait::async_trait;
use helpers::keys;
use log::info;
use redis::{aio::ConnectionLike, AsyncCommands};
use resources::{with_redis_resource, with_shared_redis_resource, ResourceManager};
use scheduling::{Job, TaskManager};
use std::cmp::Ordering;
use uuid::Uuid;

#[derive(Clone)]
pub struct SlotCountAdjusterJob {
    slot_count: usize,
}

#[async_trait]
impl Job for SlotCountAdjusterJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);

        subjobs::adjust_slots(&manager, &mut con, self.slot_count).await?;
        manager.ready().await;

        // This job continues running as a dummy so it will be re-run if the redis connection dies
        manager.termination_signal().await;
        Ok(())
    }
}

impl SlotCountAdjusterJob {
    pub fn new(slot_count: usize) -> Self {
        Self { slot_count }
    }
}

mod subjobs {
    use super::*;

    pub async fn adjust_slots<C: AsyncCommands + ConnectionLike>(
        manager: &TaskManager<Context>,
        con: &mut C,
        target: usize,
    ) -> Result<()> {
        let orchestrator_id = manager.context.id.clone();

        let current: usize = con
            .scard(keys::orchestrator::slots::allocated(&orchestrator_id))
            .await?;

        if target != current {
            info!("Adjusting slot amount from {} -> {}", current, target);
        }

        match target.cmp(&current) {
            Ordering::Greater => {
                let delta = target - current;
                for _ in 0..delta {
                    let slot_id = Uuid::new_v4().to_hyphenated().to_string();

                    redis::pipe()
                        .atomic()
                        .cmd("SADD")
                        .arg(keys::orchestrator::slots::allocated(&orchestrator_id))
                        .arg(&slot_id)
                        .cmd("RPUSH")
                        .arg(keys::orchestrator::slots::reclaimed(&orchestrator_id))
                        .arg(&slot_id)
                        .query_async(con)
                        .await?;
                }
            }
            Ordering::Less => {
                // We need a separate connection here to prevent the brpop command from blocking other jobs
                let mut blockable_con = with_redis_resource!(manager);
                let delta = current - target;
                for _ in 0..delta {
                    let (_, slot_id): (String, String) = blockable_con
                        .brpop(keys::orchestrator::slots::available(&orchestrator_id), 0)
                        .await?;
                    con.srem::<_, _, ()>(
                        keys::orchestrator::slots::allocated(&orchestrator_id),
                        &slot_id,
                    )
                    .await?;
                }
            }
            Ordering::Equal => {}
        };

        let slots: Vec<String> = con
            .smembers(keys::orchestrator::slots::allocated(&orchestrator_id))
            .await?;
        info!("Managed slots: \n\t{:?}", slots.join("\n\t"));

        Ok(())
    }
}
