use crate::Context;
use anyhow::Result;
use async_trait::async_trait;
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

        subjobs::register(&mut con).await?;
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
    use helpers::{env, keys, ServicePort};
    use redis::{aio::ConnectionLike, AsyncCommands};

    pub async fn register<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Result<()> {
        let data = [
            ("host", (*env::service::manager::HOST).to_owned()),
            ("port", ServicePort::Manager.port().to_string()),
        ];

        con.hset_multiple(&(*keys::manager::METADATA), &data)
            .await?;
        con.sadd(&(*keys::manager::LIST), &(*env::service::manager::ID))
            .await?;

        Ok(())
    }

    pub async fn deregister<C: AsyncCommands + ConnectionLike>(con: &mut C) -> Result<()> {
        con.srem::<_, _, ()>(&(*keys::manager::LIST), &(*env::service::manager::ID))
            .await?;

        con.del(&(*keys::manager::METADATA)).await?;

        Ok(())
    }
}
