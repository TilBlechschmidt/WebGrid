use super::Options;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};
use crate::libraries::{
    lifecycle::{BeatValue, HeartBeat},
    metrics::MetricsProcessor,
};
use crate::libraries::{resources::DefaultResourceManager, storage::StorageHandler};
use anyhow::Result;

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub storage_id: String,
    pub metrics: MetricsProcessor<Self, DefaultResourceManager>,
    pub storage: StorageHandler,
}

impl Context {
    pub async fn new(
        options: &Options,
        redis_url: String,
        storage_id: String,
        provider_id: String,
        size_threshold: f64,
        cleanup_target: f64,
    ) -> Result<Self> {
        let addr = format!("{}:{}", options.host, options.port);
        let heart_beat = HeartBeat::with_value(BeatValue::Constant(addr));

        heart_beat
            .add_beat(&keys::storage::host(&storage_id, &provider_id), 60, 120)
            .await;

        let storage = StorageHandler::new(
            options.storage_directory.clone(),
            size_threshold,
            cleanup_target,
        )
        .await?;

        Ok(Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat,
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
