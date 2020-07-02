use async_trait::async_trait;
use core::pin::Pin;
use futures::Stream;
use redis::{aio::ConnectionLike, Msg, RedisError, RedisResult};
use scheduling::TaskResourceHandle;
use thiserror::Error;

pub enum PubSubResourceError {
    StreamClosed,
}

#[derive(Error, Debug)]
pub enum ResourceManagerError {
    #[error("failed to connect to redis")]
    Redis(#[from] RedisError),
}

pub type ResourceManagerResult<T> = Result<T, ResourceManagerError>;
pub type PubSub = Box<dyn PubSubResource + Send>;

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

#[async_trait]
pub trait PubSubResource {
    async fn psubscribe(&mut self, pchannel: &str) -> RedisResult<()>;

    fn on_message<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Msg, PubSubResourceError>> + Send + 'a>>;
}
