use super::super::super::super::BoxedError;
use async_trait::async_trait;
use futures::stream::BoxStream;
use redis::aio::ConnectionLike;
use redis::{Msg, RedisResult};
use thiserror::Error;

/// Variant for redis connections
pub enum RedisConnectionVariant {
    /// Individual connection that may allow for blocking commands without disturbing other users.
    /// While it may be reused after going out-of-scope, this variant indicates that the consumer
    /// is operating long-running, blocking operations on the connection and the use of a resource pool is
    /// unadvisable as it may take a long time for the connection to be returned.
    Owned,
    /// Same as [`Owned`](RedisConnectionVariant::Owned) but indicates that the consumer is expected to only
    /// block for relatively short periods of time (e.g. waiting for responses to requests while processing a
    /// [`Notification`](super::super::super::event::Notification)) so that the use of a connection pool is viable.
    Pooled,
    /// Connection that can be shared between multiple users and generally does not permit blocking commands
    Multiplexed,
}

/// Errors that may occur while listening on a [`PubSubResource`]
#[derive(Error, Debug)]
pub enum PubSubResourceError {
    /// Underlying stream has been closed
    #[error("redis stream has been closed")]
    StreamClosed,
}

/// Wrapper trait for [`PubSub`](redis::aio::PubSub) to allow for black-box implementation
#[async_trait]
pub trait PubSubResource {
    /// Subscribe to a channel using a wildcard pattern
    async fn psubscribe(&mut self, pchannel: &str) -> RedisResult<()>;
    /// Subscribe to a channel by name
    async fn subscribe(&mut self, channel: &str) -> RedisResult<()>;

    /// Listen to a channel for incoming messages
    fn into_on_message<'a>(self) -> BoxStream<'a, Result<Msg, PubSubResourceError>>;
}

/// Factory for redis connections of different [types](RedisConnectionVariant)
#[async_trait]
pub trait RedisFactory {
    /// Type returned when creating a PubSub connection
    type PubSub: PubSubResource;

    /// Creates a new PubSub connection
    async fn pubsub(&self) -> Result<Self::PubSub, BoxedError>;

    /// Establishes a new connection, retrieves one from a pool, or clones a shared one
    async fn connection(
        &self,
        variant: RedisConnectionVariant,
    ) -> Result<Box<dyn ConnectionLike + Send + Sync>, BoxedError>;
}
