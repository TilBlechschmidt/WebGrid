//! Metadata aggregator which builds and persists objects for each session

mod options;
mod services;

use async_trait::async_trait;
use harness::{Heart, Module, ServiceRunner};
use jatsl::{schedule, JobScheduler};
use library::communication::event::{
    ConsumerGroupDescriptor, ConsumerGroupIdentifier, QueueLocation,
};
use library::BoxedError;

pub use options::Options;
use services::*;
use tracing::{debug, instrument};

/// Module implementation
pub struct Collector {
    options: Options,
}

impl Collector {
    /// Creates a new instance from raw parts
    pub fn new(options: Options) -> Self {
        Self { options }
    }
}

#[async_trait]
impl Module for Collector {
    #[instrument(skip(self, scheduler))]
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let redis_url = self.options.redis.url.clone();
        let group =
            ConsumerGroupDescriptor::new(ConsumerGroupIdentifier::Collector, QueueLocation::Tail);
        let consumer = self.options.queueing.id.to_string();

        debug!("Acquiring mongo connection");
        // TODO The database should not be instantiated in advance. Instead a resource pool should be used for proper error handling!
        let database = self.options.mongo.database().await?;
        let collection = self.options.mongo.collection(&database).await?;
        let staging_collection = self.options.mongo.staging_collection(&database).await?;

        let creation_watcher = ServiceRunner::<CreationWatcherService>::new(
            redis_url.clone(),
            group.clone(),
            consumer.clone(),
            staging_collection.clone(),
        );

        let scheduling_watcher = ServiceRunner::<SchedulingWatcherService>::new(
            redis_url.clone(),
            group.clone(),
            consumer.clone(),
            staging_collection.clone(),
        );

        let provisioning_watcher = ServiceRunner::<ProvisioningWatcherService>::new(
            redis_url.clone(),
            group.clone(),
            consumer.clone(),
            staging_collection.clone(),
        );

        let operational_watcher = ServiceRunner::<OperationalWatcherService>::new(
            redis_url.clone(),
            group.clone(),
            consumer.clone(),
            staging_collection.clone(),
        );

        let metadata_watcher = ServiceRunner::<MetadataWatcherService>::new(
            redis_url.clone(),
            group.clone(),
            consumer.clone(),
            staging_collection.clone(),
        );

        let termination_watcher = ServiceRunner::<TerminationWatcherService>::new(
            redis_url.clone(),
            group.clone(),
            consumer.clone(),
            (collection, staging_collection.clone()),
        );

        debug!("Scheduling jobs");
        schedule!(scheduler, {
            creation_watcher,
            scheduling_watcher,
            provisioning_watcher,
            operational_watcher,
            metadata_watcher,
            termination_watcher
        });

        Ok(Some(Heart::without_heart_stone()))
    }
}
