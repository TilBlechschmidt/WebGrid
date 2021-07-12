use super::super::super::super::EmptyResult;
use super::super::super::event::{
    QueueDescriptor, QueueDescriptorExtension, RawNotificationPublisher,
};
use super::super::super::request::{RawResponsePublisher, ResponseLocation};
use super::super::json::{JsonNotificationPublisher, JsonResponsePublisher};
use super::RESPONSE_KEY_PREFIX;
use super::{RedisConnectionVariant, RedisFactory};
use async_trait::async_trait;
use redis::streams::StreamMaxlen;
use redis::AsyncCommands;

use super::STREAM_ID_NEW;
use super::STREAM_PAYLOAD_KEY;

/// Multi-purpose publisher implementation using redis
///
/// - [`NotificationPublisher`](super::super::super::event::NotificationPublisher) implementation using [`XADD`](https://redis.io/commands/xadd)
/// - [`ResponsePublisher`](super::super::super::request::ResponsePublisher) implementation using [`RPUSH`](https://redis.io/commands/rpush)
#[derive(Clone)]
pub struct RedisPublisher<F: RedisFactory> {
    factory: F,
}

impl<F> RedisPublisher<F>
where
    F: RedisFactory,
{
    /// Creates a new instance from an existing redis connection
    pub fn new(factory: F) -> Self {
        Self { factory }
    }
}

impl<F> JsonNotificationPublisher for RedisPublisher<F> where F: RedisFactory + Send + Sync {}
impl<F> JsonResponsePublisher for RedisPublisher<F> where F: RedisFactory + Send + Sync {}

#[async_trait]
impl<F> RawNotificationPublisher for RedisPublisher<F>
where
    F: RedisFactory + Send + Sync,
{
    async fn publish_raw(
        &self,
        data: &[u8],
        descriptor: QueueDescriptor,
        extension: Option<QueueDescriptorExtension>,
    ) -> EmptyResult {
        let limit = StreamMaxlen::Approx(descriptor.limit());
        let key = match extension {
            Some(extension) => descriptor.key_with_extension(&extension),
            None => descriptor.key().to_owned(),
        };

        let mut con = self
            .factory
            .connection(RedisConnectionVariant::Multiplexed)
            .await?;

        con.xadd_maxlen::<_, _, _, _, ()>(key, limit, STREAM_ID_NEW, &[(STREAM_PAYLOAD_KEY, data)])
            .await?;

        Ok(())
    }
}

#[async_trait]
impl<F> RawResponsePublisher for RedisPublisher<F>
where
    F: RedisFactory + Send + Sync,
{
    async fn publish_raw(&self, data: &[u8], location: ResponseLocation) -> EmptyResult {
        let key = format!("{}{}", RESPONSE_KEY_PREFIX, location);
        let mut con = self
            .factory
            .connection(RedisConnectionVariant::Multiplexed)
            .await?;

        con.rpush(key, data).await?;

        Ok(())
    }
}
