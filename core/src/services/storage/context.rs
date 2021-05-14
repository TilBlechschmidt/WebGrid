use super::Options;
use crate::libraries::metrics::MetricsProcessor;
use crate::libraries::resources::ResourceManagerProvider;
use crate::libraries::{resources::DefaultResourceManager, storage::StorageHandler};
use anyhow::Result;
use uuid::Uuid;

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub storage_id: Uuid,
    pub metrics: MetricsProcessor<Self, DefaultResourceManager>,
    pub storage: StorageHandler,
}

impl Context {
    pub async fn new(
        options: &Options,
        redis_url: String,
        storage_id: Uuid,
        size_threshold: f64,
        cleanup_target: f64,
    ) -> Result<Self> {
        let storage = StorageHandler::new(
            options.storage_directory.clone(),
            size_threshold,
            cleanup_target,
        )
        .await?;

        Ok(Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            storage_id,
            metrics: MetricsProcessor::default(),
            storage,
        })
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
