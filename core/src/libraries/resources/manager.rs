use super::{
    RedisResource, ResourceManager, ResourceManagerResult, SharedRedisResource,
    StandaloneRedisResource,
};
use crate::libraries::scheduling::TaskResourceHandle;
use async_trait::async_trait;

#[derive(Clone)]
pub struct DefaultResourceManager {
    redis_url: String,
}

impl DefaultResourceManager {
    pub fn new(redis_url: String) -> Self {
        Self { redis_url }
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
