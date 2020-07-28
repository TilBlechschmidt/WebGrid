use super::super::Context;
use crate::libraries::resources::ResourceManager;
use crate::libraries::scheduling::{Job, TaskManager};
use crate::with_shared_redis_resource;
use anyhow::Result;
use async_trait::async_trait;

#[derive(Clone)]
pub struct RegistrationJob {
    id: String,
    host: String,
    port: u16,
}

#[async_trait]
impl Job for RegistrationJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);

        subjobs::register(&mut con, &self.id, self.host.clone(), self.port).await?;
        manager.ready().await;

        manager.termination_signal().await;
        subjobs::deregister(&mut con, &self.id).await?;

        Ok(())
    }
}

impl RegistrationJob {
    pub fn new(id: String, host: String, port: u16) -> Self {
        Self { id, host, port }
    }
}

mod subjobs {
    use super::*;
    use crate::libraries::helpers::keys;
    use redis::{aio::ConnectionLike, AsyncCommands};

    pub async fn register<C: AsyncCommands + ConnectionLike>(
        con: &mut C,
        id: &str,
        host: String,
        port: u16,
    ) -> Result<()> {
        let data = [("host", host), ("port", port.to_string())];

        con.hset_multiple(keys::manager::metadata(id), &data)
            .await?;
        con.sadd(&(*keys::manager::LIST), id).await?;

        Ok(())
    }

    pub async fn deregister<C: AsyncCommands + ConnectionLike>(
        con: &mut C,
        id: &str,
    ) -> Result<()> {
        con.srem::<_, _, ()>(&(*keys::manager::LIST), id).await?;

        con.del(keys::manager::metadata(id)).await?;

        Ok(())
    }
}
