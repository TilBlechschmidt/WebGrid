use super::super::super::BoxedError;
use super::{ConsumerGroupDescriptor, QueueDescriptor, QueueDescriptorExtension, QueueEntry};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::time::Duration;

/// Allows consumption of notification queues using [consumer groups](ConsumerGroupDescriptor)
#[async_trait]
pub trait QueueProvider {
    /// Type of [`QueueEntry`] returned by the provider
    type Entry: QueueEntry + Send + Sync;

    /// Subscribes to new notifications on a given queue joining the specified [`ConsumerGroup`](ConsumerGroupDescriptor)
    /// with the given [`ConsumerIdentifier`] or creates it if it does not exist.
    async fn consume(
        &self,
        queue: QueueDescriptor,
        group: &ConsumerGroupDescriptor,
        consumer: &str, // &ConsumerIdentifier
        batch_size: usize,
        idle_timeout: Option<Duration>,
        extension: &Option<QueueDescriptorExtension>,
    ) -> Result<BoxStream<Result<Self::Entry, BoxedError>>, BoxedError>;
}
