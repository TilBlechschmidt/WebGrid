//! GraphQL based interface to extract metadata from the grid

use async_trait::async_trait;

mod options;
mod schema;
mod server;

use jatsl::{schedule, JobScheduler};
pub use options::Options;
use tracing::{debug, instrument};

use domain::WebgridServiceDescriptor;
use harness::{Heart, Module, RedisServiceAdvertisementJob};
use library::BoxedError;

use self::server::ServerJob;

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
    #[instrument(skip(self, scheduler))]
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        debug!("Acquiring mongo connection");
        // TODO The database should not be instantiated in advance. Instead a resource pool should be used for proper error handling!
        let database = self.options.mongo.database().await?;
        let storage_collection = self.options.mongo.collection(&database).await?;
        let staging_collection = self.options.mongo.staging_collection(&database).await?;

        let server_job = ServerJob::new(
            crate::constants::PORT_API,
            self.options.web_root.clone(),
            storage_collection,
            staging_collection,
        );
        let advertise_job = self.build_advertise_job();

        debug!("Scheduling jobs");
        schedule!(scheduler, { advertise_job, server_job });

        Ok(Some(Heart::without_heart_stone()))
    }
}
