use crate::libraries::{helpers::keys, lifecycle::BeatValue};
use crate::libraries::{lifecycle::HeartBeat, resources::ResourceManagerProvider};
use crate::libraries::{metrics::MetricsProcessor, resources::DefaultResourceManager};
use std::ops::Deref;

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub metrics: MetricsProcessor<Self, DefaultResourceManager>,
}

impl Context {
    pub async fn new(redis_url: String, host: String, id: &str) -> Self {
        let heart_beat = HeartBeat::with_value(BeatValue::Constant(host));
        heart_beat.add_beat(&keys::manager::host(id), 60, 120).await;

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat,
            metrics: MetricsProcessor::default(),
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}

pub struct SessionCreationContext {
    context: Context,
    pub remote_addr: String,
    pub user_agent: String,
    pub capabilities: String,
}

impl SessionCreationContext {
    pub fn new(
        context: Context,
        remote_addr: String,
        user_agent: String,
        capabilities: String,
    ) -> Self {
        Self {
            context,
            remote_addr,
            user_agent,
            capabilities,
        }
    }
}

impl Deref for SessionCreationContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
