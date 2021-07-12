use super::super::super::super::BoxedError;
use super::super::json::JsonQueueEntry;
use super::RedisQueueError;
use super::STREAM_PAYLOAD_KEY;
use crate::library::communication::event::RawQueueEntry;
use async_trait::async_trait;
use redis::aio::ConnectionLike;
use redis::streams::StreamId;
use redis::AsyncCommands;

/// Redis based implementation of the [`QueueEntry`](crate::library::communication::event::QueueEntry) trait
pub struct RedisQueueEntry<C> {
    con: C,
    id: String,
    key: String,
    group: String,
    payload: Vec<u8>,
}

impl<C> RedisQueueEntry<C>
where
    C: ConnectionLike + Send + Sync,
{
    pub(super) fn new(
        con: C,
        entry: StreamId,
        key: String,
        group: String,
    ) -> Result<Self, RedisQueueError> {
        let payload = entry
            .get(STREAM_PAYLOAD_KEY)
            .ok_or(RedisQueueError::MissingPayload)?;

        Ok(Self {
            con,
            id: entry.id,
            key,
            group,
            payload,
        })
    }
}

#[async_trait]
impl<C> RawQueueEntry for RedisQueueEntry<C>
where
    C: ConnectionLike + Send + Sync,
{
    fn payload(&self) -> &[u8] {
        &self.payload
    }

    async fn acknowledge(&mut self) -> Result<(), BoxedError> {
        self.con
            .xack::<_, _, _, ()>(&self.key, &self.group, &[&self.id])
            .await?;

        Ok(())
    }
}

impl<C> JsonQueueEntry for RedisQueueEntry<C> where C: ConnectionLike + Send + Sync {}
