use crate::Context;
use anyhow::Result;
use async_trait::async_trait;
use redis::{aio::ConnectionLike, AsyncCommands};
use resources::{with_shared_redis_resource, ResourceManager};
use scheduling::{Job, TaskManager};

#[derive(Clone)]
pub struct RegistrationJob {}

#[async_trait]
impl Job for RegistrationJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);

        subjobs::register(&mut con, &manager.context).await?;
        manager.ready().await;

        manager.termination_signal().await;
        subjobs::deregister(&mut con).await?;

        Ok(())
    }
}

impl RegistrationJob {
    pub fn new() -> Self {
        Self {}
    }
}

mod subjobs {
    use super::*;
    use helpers::{env, keys};

    pub async fn register<C: AsyncCommands + ConnectionLike>(
        con: &mut C,
        context: &Context,
    ) -> Result<()> {
        let orchestrator_id: String = (*env::service::orchestrator::ID).clone();
        let type_str = format!("{}", context.provisioner_type);
        let capabilities = context.provisioner.capabilities();

        con.set::<_, _, ()>(
            keys::orchestrator::capabilities::platform_name(&orchestrator_id),
            &capabilities.platform_name,
        )
        .await
        .unwrap();
        if !capabilities.browsers.is_empty() {
            con.sadd::<_, _, ()>(
                keys::orchestrator::capabilities::browsers(&orchestrator_id),
                capabilities.browsers,
            )
            .await
            .unwrap();
        }

        con.hset_multiple::<_, _, _, ()>(
            keys::orchestrator::metadata(&orchestrator_id),
            &[("type", type_str)],
        )
        .await
        .unwrap();
        con.sadd::<_, _, ()>(&(*keys::orchestrator::LIST), &orchestrator_id)
            .await
            .unwrap();

        Ok(())
    }

    pub async fn deregister<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Result<()> {
        let orchestrator_id: String = (*env::service::orchestrator::ID).clone();

        con.srem::<_, _, ()>(&(*keys::orchestrator::LIST), &orchestrator_id)
            .await
            .unwrap();

        con.del::<_, ()>(&[
            keys::orchestrator::metadata(&orchestrator_id),
            keys::orchestrator::capabilities::platform_name(&orchestrator_id),
            keys::orchestrator::capabilities::browsers(&orchestrator_id),
        ])
        .await
        .unwrap();

        Ok(())
    }
}
