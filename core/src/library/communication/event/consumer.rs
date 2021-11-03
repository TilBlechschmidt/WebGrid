use super::super::super::EmptyResult;
use super::{ConsumerGroupDescriptor, QueueDescriptorExtension};
use super::{Notification, NotificationFrame};
use super::{QueueEntry, QueueProvider, RawQueueEntry};
use crate::library::BoxedError;
use async_trait::async_trait;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};

const DEFAULT_BATCH_SIZE: usize = 50;
const DEFAULT_CONCURRENCY: usize = DEFAULT_BATCH_SIZE;
const DEFAULT_IDLE_TIMEOUT: Option<Duration> = None;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
enum ConsumerError {
    #[error("failed to receive item")]
    ItemReceptionFailed(#[source] BoxedError),
    #[error("unable to parse payload")]
    PayloadParseFailed(#[source] BoxedError),
    #[error("notification frame consumption failed")]
    ConsumptionFailed(#[source] BoxedError),
    #[error("could not acknowledge queue entry")]
    AckFailed(#[source] BoxedError),
}

/// Entity which may consume and process [`Notifications`](Notification)
#[async_trait]
pub trait Consumer {
    /// Notification to consume
    type Notification: Notification;

    /// Processes an event notification and returns whether it succeeded or failed
    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult;
}

/// Helper functions to aid the consumption of messages
#[async_trait]
pub trait ConsumerExt {
    /// Handles individual queue entries by parsing their payload and handind them to the underlying consumer.
    async fn recv_queue_item<Q>(
        &self,
        item: Result<<Q as QueueProvider>::Entry, BoxedError>,
        seq_id: u64,
    ) -> EmptyResult
    where
        Q: QueueProvider + Send + Sync;

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
    #[instrument(err, skip(self, item, _seq_id), fields(seq_id = _seq_id, notification = ?type_name::<C::Notification>()))]
    async fn recv_queue_item<Q>(
        &self,
        item: Result<<Q as QueueProvider>::Entry, BoxedError>,
        _seq_id: u64,
    ) -> EmptyResult
    where
        Q: QueueProvider + Send + Sync,
    {
        debug!("Processing incoming queue item");
        let mut entry = item.map_err(|e| ConsumerError::ItemReceptionFailed(e))?;

        trace!("Parsing payload");
        let frame = entry
            .parse_payload::<C::Notification>()
            .map_err(|e| ConsumerError::PayloadParseFailed(e))?;

        trace!(?frame, "Consuming frame");
        self.consume(frame)
            .await
            .map_err(|e| ConsumerError::ConsumptionFailed(e))?;

        trace!("Acknowledging queue item");
        entry
            .acknowledge()
            .await
            .map_err(|e| ConsumerError::AckFailed(e))?;

        Ok(())
    }

    #[instrument(err, skip(self, provider, group), fields(group = ?group.identifier(), key = ?C::Notification::queue().key()))]
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

        let seq = Arc::new(AtomicU64::new(0));

        debug!("Starting to consume queue");
        stream
            .for_each_concurrent(Some(DEFAULT_CONCURRENCY), |item| async {
                let seq_id = seq.fetch_add(1, Ordering::Relaxed);
                if let Err(error) = self.recv_queue_item::<Q>(item, seq_id).await {
                    error!(seq_id, ?error, "Failed to process queue item");
                }
            })
            .await;

        Ok(())
    }
}
