//! HTTP traffic ingress for all non-internal interactions with the grid

// Trivia: This module was supposed to be called "gateway". However, some bug in rust-analyzer
//         prevented a module with that name from being recognized as part of the project, breaking
//         most IDE functionality. For that reason, the module is now dubbed "gangway" 🤷‍♂️

use std::time::Duration;

use crate::gangway::created_publisher::CreatedNotificationPublisherJob;
use crate::gangway::proxy::ProxyJob;
use async_trait::async_trait;
use domain::WebgridServiceDescriptor;
use harness::{Heart, Module, RedisServiceDiscoveryJob, ServiceRunner};
use jatsl::{schedule, Job, JobScheduler};
use library::communication::discovery::pubsub::PubSubServiceDiscoverer;
use library::communication::event::{
    ConsumerGroupDescriptor, ConsumerGroupIdentifier, QueueLocation,
};
use library::BoxedError;

mod created_publisher;
mod options;
mod proxy;
mod services;

use library::storage::s3::S3StorageBackend;
pub use options::Options;
use services::*;
use tokio::time::sleep;
use tracing::debug;

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
        let group = ConsumerGroupDescriptor::new(
            ConsumerGroupIdentifier::Gangway(self.options.queueing.id.clone()),
            QueueLocation::Tail,
        );
        let operational_runner = ServiceRunner::<OperationalListenerService<_>>::new(
            redis_url.clone(),
            group.clone(),
            identifier.clone(),
            creation_handle.clone(),
        );
        let failure_runner = ServiceRunner::<TerminationListenerService<_>>::new(
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
            self.options.storage.backend.clone(),
        );

        // Schedule everything
        debug!("Scheduling jobs");
        schedule!(scheduler, {
            discovery_job,
            proxy_job,
            created_publisher_job,
            operational_runner,
            failure_runner
        });

        Ok(Some(Heart::without_heart_stone()))
    }

    async fn pre_shutdown(&mut self, scheduler: &JobScheduler) {
        // Give e.g. K8s some time to discover that we are not ready
        debug!("Postponing shutdown for readiness probe to be observed");
        sleep(Duration::from_secs(5)).await;

        // Terminate the proxy server gracefully
        debug!("Gracefully shutting down HTTP server");
        scheduler.terminate_job(
            &ProxyJob::<PubSubServiceDiscoverer<WebgridServiceDescriptor>, S3StorageBackend>::NAME
                .into(),
            self.options.termination_grace_period,
        ).await;
    }
}
