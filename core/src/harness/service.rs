use super::RedisCommunicationFactory;
use crate::library::communication::event::{
    ConsumerExt, ConsumerGroupDescriptor, QueueDescriptorExtension,
};
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;
use jatsl::{Job, JobManager};
use std::sync::Arc;

/// Structure which can be instantiated with a [`CommunicationFactory`]
pub trait Service<F: CommunicationFactory + Send + Sync> {
    /// Name of the service displayed in log messages
    const NAME: &'static str;
    /// Instance type which will be instantiated
    type Instance: Send + Sync;
    /// Configuration type passed to the service
    type Config: Send + Sync;

    /// Creates a new instance which could be of a different type.
    /// This is common when `Self: OptionalRequestProcessor` where this
    /// function would return an instance of [`Responder`](crate::library::communication::request::Responder)
    /// containing an instance of `Self`.
    fn instantiate(factory: F, config: &Self::Config) -> Self::Instance;
}

/// Runner for [`Service`] implementations where [`Service::Instance`] is conforming to the [`ConsumerExt`] trait
pub struct ServiceRunner<S: Service<RedisCommunicationFactory>> {
    redis_url: String,
    group: ConsumerGroupDescriptor,
    consumer: String,
    config: <S as Service<RedisCommunicationFactory>>::Config,
    extension: Option<QueueDescriptorExtension>,
}

impl<S> ServiceRunner<S>
where
    S: Service<RedisCommunicationFactory>,
    S::Instance: ConsumerExt + Send + Sync,
{
    /// Creates a new runner job which will connect to the given redis server and use the provided consumer group and name.
    pub fn new(
        redis_url: String,
        group: ConsumerGroupDescriptor,
        consumer: String,
        config: <S as Service<RedisCommunicationFactory>>::Config,
    ) -> Self {
        Self::new_with_extension(redis_url, None, group, consumer, config)
    }

    /// Same as [`ServiceRunner::new`] but with a [`QueueDescriptorExtension`]
    pub fn new_with_extension(
        redis_url: String,
        extension: Option<QueueDescriptorExtension>,
        group: ConsumerGroupDescriptor,
        consumer: String,
        config: <S as Service<RedisCommunicationFactory>>::Config,
    ) -> Self {
        Self {
            redis_url,
            group,
            consumer,
            config,
            extension,
        }
    }
}

#[async_trait]
impl<S> Job for ServiceRunner<S>
where
    S: Service<RedisCommunicationFactory> + Send + Sync,
    S::Instance: ConsumerExt,
{
    const NAME: &'static str = "ServiceRunner";

    fn name(&self) -> String {
        format!("{}({})", Self::NAME, S::NAME)
    }

    async fn execute(&self, manager: JobManager) -> EmptyResult {
        let handle_provider = Arc::new(manager.clone());
        let factory = RedisCommunicationFactory::new(self.redis_url.clone(), handle_provider);
        let provider = factory.queue_provider();
        let service = S::instantiate(factory, &self.config);

        // TODO Wait for the factory to be able to provide alive connections!
        //      Maybe by adding a "ready()" method to it that under the hood runs a ping against the server.
        manager.ready().await;

        service
            .consume_queue(provider, &self.group, &self.consumer, &self.extension)
            .await?;

        Ok(())
    }
}
