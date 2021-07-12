//! HTTP traffic ingress for all non-internal interactions with the grid

// Trivia: This module was supposed to be called "gateway". However, some bug in rust-analyzer
//         prevented a module with that name from being recognized as part of the project, breaking
//         most IDE functionality. For that reason, the module is now dubbed "gangway" 🤷‍♂️

use crate::domain::WebgridServiceDescriptor;
use crate::harness::{Heart, Module, RedisServiceDiscoveryJob, ServiceRunner};
use crate::library::communication::discovery::pubsub::PubSubServiceDiscoverer;
use crate::library::communication::event::{
    ConsumerGroupDescriptor, ConsumerGroupIdentifier, QueueLocation,
};
use crate::library::BoxedError;
use crate::module::gangway::created_publisher::CreatedNotificationPublisherJob;
use crate::module::gangway::proxy::ProxyJob;
use async_trait::async_trait;
use jatsl::{schedule, JobScheduler};

mod created_publisher;
mod options;
mod proxy;
mod services;

pub use options::Options;
use services::*;

/// Module implementation
pub struct Gangway {
    options: Options,
}

impl Gangway {
    /// Creates a new instance from raw parts
    pub fn new(options: Options) -> Self {
        Self { options }
    }
}

#[async_trait]
impl Module for Gangway {
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let identifier = self.options.queueing.id.to_string();
        let redis_url = self.options.redis.url.clone();

        // Build all the required data structures
        let (creation_handle, creation_rx) = SessionCreationCommunicationHandle::new(1000);
        let (discoverer, discovery_daemon) =
            PubSubServiceDiscoverer::<WebgridServiceDescriptor>::new(
                self.options.service_discovery.cache_size,
                self.options.service_discovery.request_channel_size,
                self.options.service_discovery.response_channel_size,
            );

        // Create the ServiceRunner instances for all services
        let group =
            ConsumerGroupDescriptor::new(ConsumerGroupIdentifier::Gangway, QueueLocation::Tail);
        let operational_runner = ServiceRunner::<OperationalListenerService<_>>::new(
            redis_url.clone(),
            group.clone(),
            identifier.clone(),
            creation_handle.clone(),
        );
        let failure_runner = ServiceRunner::<FailureListenerService<_>>::new(
            redis_url.clone(),
            group,
            identifier.clone(),
            creation_handle.clone(),
        );

        // Create individual jobs
        let created_publisher_job =
            CreatedNotificationPublisherJob::new(creation_rx, redis_url.clone());
        let discovery_job = RedisServiceDiscoveryJob::new(redis_url, discovery_daemon);
        let proxy_job = ProxyJob::new(
            crate::constants::PORT_GANGWAY,
            identifier,
            discoverer,
            creation_handle,
        );

        // Schedule everything
        schedule!(scheduler, {
            discovery_job,
            proxy_job,
            created_publisher_job,
            operational_runner,
            failure_runner
        });

        Ok(Some(Heart::without_heart_stone()))
    }
}
