use std::sync::Arc;

use async_trait::async_trait;
use jatsl::{TaskManager, TaskResourceHandle};
use redis::aio::ConnectionLike;

use super::pubsub::MonitoredPubSub;
use super::resource::RedisResource;
use crate::library::communication::implementation::redis::{
    RedisConnectionVariant, RedisFactory, RedisPublisher, RedisQueueProvider,
    RedisResponseCollector, RedisServiceAdvertiser,
};
use crate::library::communication::request::CompositeRequestor;
use crate::library::communication::CommunicationFactory;
use crate::library::BoxedError;

/// [`RedisFactory`] implementation providing [`jatsl`] interop
pub struct MonitoredRedisFactory {
    url: String,
    handle_provider: BoxedResourceHandleProvider,
}

impl MonitoredRedisFactory {
    /// Creates a new factory opening connections to the given URL
    pub fn new(url: String, handle_provider: BoxedResourceHandleProvider) -> Self {
        Self {
            url,
            handle_provider,
        }
    }
}

#[async_trait]
impl RedisFactory for MonitoredRedisFactory {
    type PubSub = MonitoredPubSub;

    async fn pubsub(&self) -> Result<Self::PubSub, BoxedError> {
        let handle = self.handle_provider.create_handle();
        let resource = RedisResource::new(handle.clone(), &self.url).await?;

        Ok(MonitoredPubSub::new(resource.con, resource.handle))
    }

    async fn connection(
        &self,
        variant: RedisConnectionVariant,
    ) -> Result<Box<dyn ConnectionLike + Send + Sync>, BoxedError> {
        let handle = self.handle_provider.create_handle();

        match variant {
            // TODO Implement connection pooling
            RedisConnectionVariant::Owned | RedisConnectionVariant::Pooled => {
                Ok(Box::new(RedisResource::new(handle, &self.url).await?))
            }
            RedisConnectionVariant::Multiplexed => {
                Ok(Box::new(RedisResource::shared(handle, &self.url).await?))
            }
        }
    }
}

/// Factory to provide [`TaskResourceHandle`] instances
pub trait ResourceHandleProvider {
    /// Instantiates a new [`TaskResourceHandle`]
    fn create_handle(&self) -> TaskResourceHandle;
}

/// Stub resource handle provider
///
/// Creates new instances using [`TaskResourceHandle::stub()`] for situations where you do not need redundancy or task management
pub struct DummyResourceHandleProvider {}

impl DummyResourceHandleProvider {
    /// Creates a new instance wrapped in an [`Arc`]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

impl ResourceHandleProvider for DummyResourceHandleProvider {
    fn create_handle(&self) -> TaskResourceHandle {
        TaskResourceHandle::stub()
    }
}

impl<C> ResourceHandleProvider for TaskManager<C> {
    fn create_handle(&self) -> TaskResourceHandle {
        self.create_resource_handle()
    }
}

/// Dynamic dispatch version of [`ResourceHandleProvider`]
pub type BoxedResourceHandleProvider = Arc<dyn ResourceHandleProvider + Send + Sync>;

/// Communication factory based on [`MonitoredRedisFactory`]
pub struct RedisCommunicationFactory {
    url: String,
    handle_provider: BoxedResourceHandleProvider,
}

impl RedisCommunicationFactory {
    /// Creates a new instance which connects to the given URL and reports status using the given handle factory
    pub fn new(url: String, handle_provider: BoxedResourceHandleProvider) -> Self {
        Self {
            url,
            handle_provider,
        }
    }

    fn factory(&self) -> MonitoredRedisFactory {
        MonitoredRedisFactory::new(self.url.clone(), self.handle_provider.clone())
    }
}

impl CommunicationFactory for RedisCommunicationFactory {
    type QueueProvider = RedisQueueProvider<MonitoredRedisFactory>;
    type NotificationPublisher = RedisPublisher<MonitoredRedisFactory>;

    type Requestor = CompositeRequestor<
        RedisPublisher<MonitoredRedisFactory>,
        RedisResponseCollector<MonitoredRedisFactory>,
    >;

    type ResponseCollector = RedisResponseCollector<MonitoredRedisFactory>;
    type ResponsePublisher = RedisPublisher<MonitoredRedisFactory>;

    type ServiceAdvertiser = RedisServiceAdvertiser<MonitoredRedisFactory>;

    fn queue_provider(&self) -> Self::QueueProvider {
        Self::QueueProvider::new(self.factory())
    }

    fn notification_publisher(&self) -> Self::NotificationPublisher {
        Self::NotificationPublisher::new(self.factory())
    }

    fn requestor(&self) -> Self::Requestor {
        Self::Requestor::new(self.notification_publisher(), self.response_collector())
    }

    fn response_collector(&self) -> Self::ResponseCollector {
        Self::ResponseCollector::new(self.factory())
    }

    fn response_publisher(&self) -> Self::ResponsePublisher {
        Self::ResponsePublisher::new(self.factory())
    }

    fn service_advertiser(&self) -> Self::ServiceAdvertiser {
        Self::ServiceAdvertiser::new(self.factory())
    }
}
