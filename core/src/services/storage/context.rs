use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};
use crate::libraries::{
    lifecycle::{BeatValue, HeartBeat},
    metrics::MetricsProcessor,
};

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub storage_id: String,
    pub metrics: MetricsProcessor<Self, DefaultResourceManager>,
}

impl Context {
    pub async fn new(
        redis_url: String,
        storage_id: String,
        provider_id: String,
        host: String,
        port: u16,
    ) -> Self {
        let addr = format!("{}:{}", host, port);
        let heart_beat = HeartBeat::with_value(BeatValue::Constant(addr));

        heart_beat
            .add_beat(&keys::storage::host(&storage_id, &provider_id), 60, 120)
            .await;

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat,
            storage_id,
            metrics: MetricsProcessor::default(),
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
