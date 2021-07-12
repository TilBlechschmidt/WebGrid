use super::{super::super::EmptyResult, Notification, QueueDescriptor, QueueDescriptorExtension};
use async_trait::async_trait;

/// Structure which allows publishing of serialized data into a queue
#[async_trait]
pub trait RawNotificationPublisher {
    /// Sends an opaque payload to a [`Queue`](QueueDescriptor) with an optional [extension](QueueDescriptorExtension)
    async fn publish_raw(
        &self,
        data: &[u8],
        descriptor: QueueDescriptor,
        extension: Option<QueueDescriptorExtension>,
    ) -> EmptyResult;
}

/// Publisher for [`Notifications`](Notification)
#[async_trait]
pub trait NotificationPublisher {
    /// Publishes a [`Notification`] to its designated queue
    async fn publish<N: Notification + Send + Sync>(&self, notification: &N) -> EmptyResult;

    /// Publishes a [`Notification`] to its designated queue with a [`QueueDescriptorExtension`]
    async fn publish_with_extension<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: QueueDescriptorExtension,
    ) -> EmptyResult;
}
