use super::super::super::EmptyResult;
use super::Notification;
use super::{ConsumerGroupDescriptor, QueueDescriptorExtension};
use super::{QueueEntry, QueueProvider, RawQueueEntry};
use async_trait::async_trait;
use futures::StreamExt;
use log::warn;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::time::Duration;

const DEFAULT_BATCH_SIZE: usize = 10;
const DEFAULT_CONCURRENCY: usize = DEFAULT_BATCH_SIZE;
const DEFAULT_IDLE_TIMEOUT: Option<Duration> = None;

/// Entity which may consume and process [`Notifications`](Notification)
#[async_trait]
pub trait Consumer {
    /// Notification to consume
    type Notification: Notification;

    /// Processes an event notification and returns whether it succeeded or failed
    async fn consume(&self, notification: Self::Notification) -> EmptyResult;
}

/// Helper functions to aid the consumption of messages
#[async_trait]
pub trait ConsumerExt {
    /// Consumes notifications from a queue using the given provider and acknowledges
    /// those that have been successfully processed.
    async fn consume_queue<Q>(
        &self,
        provider: Q,
        group: &ConsumerGroupDescriptor,
        consumer: &str, // &ConsumerIdentifier
        extension: &Option<QueueDescriptorExtension>,
    ) -> EmptyResult
    where
        Q: QueueProvider + Send + Sync;
}

#[async_trait]
impl<'a, C> ConsumerExt for C
where
    C: Consumer + Send + Sync,
    C::Notification: DeserializeOwned + Send + Sync,
{
    async fn consume_queue<Q>(
        &self,
        provider: Q,
        group: &ConsumerGroupDescriptor,
        consumer: &str, // &ConsumerIdentifier
        extension: &Option<QueueDescriptorExtension>,
    ) -> EmptyResult
    where
        Q: QueueProvider + Send + Sync,
    {
        let stream = provider
            .consume(
                C::Notification::queue(),
                group,
                consumer,
                DEFAULT_BATCH_SIZE,
                DEFAULT_IDLE_TIMEOUT,
                extension,
            )
            .await?;

        stream
            .for_each_concurrent(Some(DEFAULT_CONCURRENCY), |item| async move {
                match item {
                    Ok(mut entry) => match entry.parse_payload::<C::Notification>() {
                        Ok(notification) => match self.consume(notification).await {
                            // TODO These warnings should also be attached to the tracing span
                            Ok(_) => {
                                if let Err(e) = entry.acknowledge().await {
                                    warn!(
                                        "Failed to acknowledge {}: {}",
                                        type_name::<C::Notification>(),
                                        e
                                    )
                                }
                            }
                            Err(e) => warn!(
                                "Failed to consume {}: {}",
                                type_name::<C::Notification>(),
                                e
                            ),
                        },
                        Err(e) => warn!(
                            "Failed to deserialize {}: {}",
                            type_name::<C::Notification>(),
                            e
                        ),
                    },
                    Err(e) => warn!(
                        "Failed to receive notification {}: {}",
                        type_name::<C::Notification>(),
                        e
                    ),
                }
            })
            .await;

        Ok(())
    }
}
