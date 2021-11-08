//! Schedules newly created sessions, assigning them to provisioners

mod options;
mod scheduling;

use std::collections::HashSet;

use async_trait::async_trait;
use harness::{Heart, Module, ServiceRunner};
use jatsl::JobScheduler;
use library::communication::event::ConsumerGroupDescriptor;
use library::BoxedError;
use scheduling::SchedulingService;

pub use options::Options;
use tracing::{debug, instrument};

/// Module implementation
pub struct Manager {
    options: Options,
}

impl Manager {
    /// Creates a new instance from raw parts
    pub fn new(options: Options) -> Self {
        Self { options }
    }
}

#[async_trait]
impl Module for Manager {
    #[instrument(skip(self, scheduler))]
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let redis_url = self.options.redis.url.clone();
        let group = ConsumerGroupDescriptor::default();
        let consumer = self.options.queueing.id.to_string();

        let runner =
            ServiceRunner::<SchedulingService<_>>::new(redis_url, group, consumer, HashSet::new());

        debug!("Scheduling service");
        scheduler.spawn_job(runner).await;

        Ok(Some(Heart::without_heart_stone()))
    }
}
