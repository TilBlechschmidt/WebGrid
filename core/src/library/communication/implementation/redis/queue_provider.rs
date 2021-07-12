use crate::library::communication::event::QueueDescriptorExtension;

use super::super::super::super::BoxedError;
use super::super::super::event::{
    ConsumerGroupDescriptor, QueueDescriptor, QueueLocation, QueueProvider,
};
use super::{
    RedisConnectionVariant, RedisFactory, RedisQueueEntry, STREAM_ID_ADDITIONS, STREAM_ID_HEAD,
    STREAM_ID_TAIL,
};
use async_trait::async_trait;
use futures::{
    stream::{self, BoxStream},
    StreamExt,
};
use log::error;
use redis::aio::ConnectionLike;
use redis::streams::StreamId;
use redis::streams::StreamReadOptions;
use redis::streams::StreamReadReply;
use redis::AsyncCommands;
use redis::RedisResult;
use std::convert::TryInto;
use std::time::Duration;

/// Queue provider implementation using [Redis Streams](https://redis.io/topics/streams-intro)
pub struct RedisQueueProvider<F: RedisFactory + Send + Sync> {
    factory: F,
}

impl<F: RedisFactory + Send + Sync> RedisQueueProvider<F> {
    /// Creates a new instance with a given [`RedisFactory`]
    pub fn new(factory: F) -> Self {
        Self { factory }
    }
}

#[async_trait]
impl<F> QueueProvider for RedisQueueProvider<F>
where
    F: RedisFactory + Send + Sync,
{
    // TODO Potentially remove this dynamic dispatch
    type Entry = RedisQueueEntry<Box<dyn ConnectionLike + Send + Sync>>;

    /// Consumes a redis stream data structure using the following steps:
    ///
    /// 1. Create the stream and/or consumer group if it does not exist
    /// 2. Start streaming entries from the PEL until the queue head is reached
    /// 3. Wait for and stream new entries in a blocking manner
    /// 4. Bail if no messages has been received within `idle_timeout` or block indefinitely
    async fn consume(
        &self,
        queue: QueueDescriptor,
        group: &ConsumerGroupDescriptor,
        consumer: &str, // &ConsumerIdentifier
        batch_size: usize,
        idle_timeout: Option<Duration>,
        extension: &Option<QueueDescriptorExtension>,
    ) -> Result<BoxStream<Result<Self::Entry, BoxedError>>, BoxedError> {
        let key = match extension {
            Some(extension) => queue.key_with_extension(extension),
            None => queue.key().to_owned(),
        };

        // Create a redis connection for the blocking XREADGROUP command
        let mut con = self
            .factory
            .connection(RedisConnectionVariant::Owned)
            .await?;

        // Create the group if it does not exist
        create_consumer_group(&mut con, &key, group).await;

        // Create the options for reading from the stream
        let block_duration = idle_timeout
            .map(|d| d.as_millis().try_into().unwrap_or_default())
            .unwrap_or_default();

        let read_options = StreamReadOptions::default()
            .group(group.identifier().to_string(), consumer)
            .count(batch_size)
            .block(block_duration);

        // Create a consumer for reading from the stream
        let entry_stream = xread_stream(con, read_options, key.clone());

        // Create an auxiliary stream that infinitely creates handles to a shared redis connection
        // It will be used to associate a connection with the QueueItems in order to acknowledge them
        let ack_con_stream = shared_redis_stream(&self.factory);

        // Combine the two streams and assemble the QueueItem from all the parts
        let stream = entry_stream
            .zip(ack_con_stream)
            .map(build_redis_queue_entry(key, group))
            .boxed();

        Ok(stream)
    }
}

fn build_redis_queue_entry(
    key: String,
    group: &ConsumerGroupDescriptor,
) -> impl Fn(
    (
        RedisResult<StreamId>,
        Result<Box<dyn ConnectionLike + Send + Sync>, BoxedError>,
    ),
) -> Result<RedisQueueEntry<Box<dyn ConnectionLike + Send + Sync>>, BoxedError> {
    let group = group.identifier().to_string();

    move |(entry, con)| {
        let entry = entry?;
        let ack_con = con?;
        let entry = RedisQueueEntry::new(ack_con, entry, key.clone(), group.clone())?;

        Ok(entry)
    }
}

async fn create_consumer_group<C: ConnectionLike + Send>(
    con: &mut C,
    key: &str,
    group: &ConsumerGroupDescriptor,
) {
    let start_id = match group.start() {
        QueueLocation::Head => STREAM_ID_HEAD,
        QueueLocation::Tail => STREAM_ID_TAIL,
    };

    con.xgroup_create_mkstream::<_, _, _, ()>(key, group.identifier().to_string(), start_id)
        .await
        .ok();
}

fn shared_redis_stream<F: RedisFactory + Send + Sync>(
    factory: &F,
) -> BoxStream<Result<Box<dyn ConnectionLike + Send + Sync>, BoxedError>> {
    stream::repeat_with(move || async move {
        factory
            .connection(RedisConnectionVariant::Multiplexed)
            .await
    })
    .then(|f| f)
    .boxed()
}

fn xread_stream<'a, C: ConnectionLike + Send + Sync + 'a>(
    con: C,
    options: StreamReadOptions,
    key: String,
) -> BoxStream<'a, RedisResult<StreamId>> {
    let initial_id: String = STREAM_ID_HEAD.to_string();

    let stream = stream::unfold((con, options, initial_id), move |(mut con, options, id)| {
        let key = key.to_owned();

        async move {
            let result = con
                .xread_options::<_, _, StreamReadReply>(&[&key], &[&id], &options)
                .await;

            match result {
                Ok(mut reply) => {
                    if let Some(stream) = reply.keys.pop() {
                        assert_eq!(stream.key, key);

                        // If we are already operating on "latest" then continue doing so
                        if id == STREAM_ID_ADDITIONS {
                            Some((Ok(stream.ids), (con, options, id)))
                        }
                        // If we are processing pending messages after a crash and have more, run through them
                        else if let Some(next_id) =
                            stream.ids.last().map(|entry| entry.id.to_owned())
                        {
                            Some((Ok(stream.ids), (con, options, next_id)))
                        }
                        // If we have finished processing pending messages after a crash, move to "latest"
                        else {
                            Some((
                                Ok(stream.ids),
                                (con, options, STREAM_ID_ADDITIONS.to_string()),
                            ))
                        }
                    } else {
                        None
                    }
                }
                Err(e) => {
                    error!("Encountered error reading from redis stream {:?}", e);
                    None
                }
            }
        }
    });

    // It is possible to stream in batches (receiving multiple entries from the redis)
    // by setting the options.count value >1. The resulting stream will still yield
    // one at a time to make it easier to use.
    stream
        .flat_map(|result| match result {
            Ok(batch) => stream::iter(batch).map(Ok).boxed(),
            Err(e) => stream::once(async { Err(e) }).boxed(),
        })
        .boxed()
}
