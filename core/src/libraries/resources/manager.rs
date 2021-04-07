use super::{
    RedisResource, ResourceManager, ResourceManagerResult, SharedRedisResource,
    StandaloneRedisResource,
};
use async_trait::async_trait;
use jatsl::TaskResourceHandle;

/// Production resource manager
///
/// Uses real redis database server
#[derive(Clone)]
pub struct DefaultResourceManager {
    redis_url: String,
}

impl DefaultResourceManager {
    /// Creates a new resource manager that connects to the redis server at the given url
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
