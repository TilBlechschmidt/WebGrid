use crate::communication::event::{QueueDescriptorExtension, QueueProvider, RawQueueEntry};
use crate::communication::implementation::json::JsonQueueEntry;
use async_trait::async_trait;

pub struct MockQueueEntry {}

#[async_trait]
impl RawQueueEntry for MockQueueEntry {
    fn payload(&self) -> &[u8] {
        unimplemented!()
    }

    async fn acknowledge(&mut self) -> crate::EmptyResult {
        unimplemented!()
    }
}

impl JsonQueueEntry for MockQueueEntry {}

pub struct MockQueueProvider {}

#[async_trait]
impl QueueProvider for MockQueueProvider {
    type Entry = MockQueueEntry;

    async fn consume(
        &self,
        _queue: crate::communication::event::QueueDescriptor,
        _group: &crate::communication::event::ConsumerGroupDescriptor,
        _consumer: &str, // &ConsumerIdentifier
        _batch_size: usize,
        _idle_timeout: Option<std::time::Duration>,
        _extension: &Option<QueueDescriptorExtension>,
    ) -> Result<futures::stream::BoxStream<Result<Self::Entry, crate::BoxedError>>, crate::BoxedError>
    {
        unimplemented!()
    }
}
