use crate::libraries::{
    metrics::MetricsProcessor,
    resources::{DefaultResourceManager, ResourceManagerProvider},
};

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub metrics: MetricsProcessor<Self, DefaultResourceManager>,
}

impl Context {
    pub fn new(redis_url: String) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            metrics: MetricsProcessor::default(),
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
