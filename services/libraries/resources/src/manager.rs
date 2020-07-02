use crate::{RedisResource, SharedRedisResource, StandaloneRedisResource};
use async_trait::async_trait;
use helpers::env;
use scheduling::TaskResourceHandle;

use crate::{ResourceManager, ResourceManagerResult};

#[derive(Clone)]
pub struct DefaultResourceManager {
    redis_url: String,
}

impl DefaultResourceManager {
    pub fn new() -> Self {
        Self {
            redis_url: env::resources::redis::URL.clone(),
        }
    }
}

#[async_trait]
impl ResourceManager for DefaultResourceManager {
    type Redis = StandaloneRedisResource;
    type SharedRedis = SharedRedisResource;

    async fn redis(&self, handle: TaskResourceHandle) -> ResourceManagerResult<Self::Redis> {
        Ok(RedisResource::new(handle, &self.redis_url).await?)
    }

    async fn shared_redis(
        &self,
        handle: TaskResourceHandle,
    ) -> ResourceManagerResult<Self::SharedRedis> {
        Ok(RedisResource::shared(handle, &self.redis_url).await?)
    }
}
