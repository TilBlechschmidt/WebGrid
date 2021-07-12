//! Schedules newly created sessions, assigning them to provisioners

mod options;
mod scheduling;

use crate::harness::{Heart, Module, ServiceRunner};
use crate::library::communication::event::ConsumerGroupDescriptor;
use crate::library::BoxedError;
use async_trait::async_trait;
use jatsl::JobScheduler;
use scheduling::SchedulingService;

pub use options::Options;

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
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let redis_url = self.options.redis.url.clone();
        let group = ConsumerGroupDescriptor::default();
        let consumer = self.options.queueing.id.to_string();

        let runner = ServiceRunner::<SchedulingService<_>>::new(redis_url, group, consumer, ());
        scheduler.spawn_job(runner);

        Ok(Some(Heart::without_heart_stone()))
    }
}
