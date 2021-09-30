//! GraphQL based interface to extract metadata from the grid

use async_trait::async_trait;

mod options;

use jatsl::{schedule, JobScheduler};
pub use options::Options;

use crate::domain::WebgridServiceDescriptor;
use crate::harness::{Heart, Module, RedisServiceAdvertisementJob};
use crate::library::BoxedError;

/// Module implementation
pub struct Api {
    options: Options,
}

impl Api {
    /// Creates a new instance from raw parts
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    fn build_advertise_job(&self) -> RedisServiceAdvertisementJob<WebgridServiceDescriptor> {
        let endpoint = format!("{}:{}", self.options.host, crate::constants::PORT_API);

        RedisServiceAdvertisementJob::new(
            self.options.redis.url.clone(),
            WebgridServiceDescriptor::Api,
            endpoint,
        )
    }
}

#[async_trait]
impl Module for Api {
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let advertise_job = self.build_advertise_job();

        schedule!(scheduler, { advertise_job });

        Ok(Some(Heart::without_heart_stone()))
    }
}
