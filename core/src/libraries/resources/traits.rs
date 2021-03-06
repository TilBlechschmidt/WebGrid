use async_trait::async_trait;
use core::pin::Pin;
use futures::Stream;
use jatsl::TaskResourceHandle;
use redis::{aio::ConnectionLike, Msg, RedisError, RedisResult};
use thiserror::Error;

/// PubSub listening errors
#[derive(Error, Debug)]
pub enum PubSubResourceError {
    #[error("redis stream has been closed")]
    StreamClosed,
}

/// Resource access errors
#[derive(Error, Debug)]
pub enum ResourceManagerError {
    #[error("failed to connect to redis")]
    Redis(#[from] RedisError),
}

/// Boxed PubSubResource shorthand
pub type PubSub = Box<dyn PubSubResource + Send>;
/// Result shorthand
pub type ResourceManagerResult<T> = Result<T, ResourceManagerError>;

/// Manager that provides access to a set of resources
#[async_trait]
pub trait ResourceManager {
    type Redis: ConnectionLike + Into<PubSub> + Send;
    type SharedRedis: ConnectionLike + Send;

    async fn redis(&self, handle: TaskResourceHandle) -> ResourceManagerResult<Self::Redis>;
    async fn shared_redis(
        &self,
        handle: TaskResourceHandle,
    ) -> ResourceManagerResult<Self::SharedRedis>;
}

/// Redis PubSub channel resource
#[async_trait]
pub trait PubSubResource {
    async fn psubscribe(&mut self, pchannel: &str) -> RedisResult<()>;
    async fn subscribe(&mut self, channel: &str) -> RedisResult<()>;

    fn on_message<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Msg, PubSubResourceError>> + Send + 'a>>;
}

pub trait ResourceManagerProvider<R: ResourceManager> {
    fn resource_manager(&self) -> R;
}

impl<R> ResourceManagerProvider<R> for R
where
    R: ResourceManager + Clone,
{
    fn resource_manager(&self) -> R {
        self.clone()
    }
}
