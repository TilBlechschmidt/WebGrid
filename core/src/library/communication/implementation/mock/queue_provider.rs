use crate::library::communication::event::{
    QueueDescriptorExtension, QueueProvider, RawQueueEntry,
};
use crate::library::communication::implementation::json::JsonQueueEntry;
use async_trait::async_trait;

pub struct MockQueueEntry {}

#[async_trait]
impl RawQueueEntry for MockQueueEntry {
    fn payload(&self) -> &[u8] {
        unimplemented!()
    }

    async fn acknowledge(&mut self) -> crate::library::EmptyResult {
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
        _queue: crate::library::communication::event::QueueDescriptor,
        _group: &crate::library::communication::event::ConsumerGroupDescriptor,
        _consumer: &str, // &ConsumerIdentifier
        _batch_size: usize,
        _idle_timeout: Option<std::time::Duration>,
        _extension: &Option<QueueDescriptorExtension>,
    ) -> Result<
        futures::stream::BoxStream<Result<Self::Entry, crate::library::BoxedError>>,
        crate::library::BoxedError,
    > {
        unimplemented!()
    }
}
