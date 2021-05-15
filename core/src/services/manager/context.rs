use super::Options;
use crate::libraries::{lifecycle::HeartBeat, resources::ResourceManagerProvider};
use crate::libraries::{metrics::MetricsProcessor, resources::DefaultResourceManager};
use opentelemetry::Context as TelemetryContext;
use std::ops::Deref;

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub metrics: MetricsProcessor<Self, DefaultResourceManager>,
    pub options: Options,
}

impl Context {
    pub async fn new(redis_url: String, options: Options) -> Self {
        let heart_beat = HeartBeat::new();

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat,
            metrics: MetricsProcessor::default(),
            options,
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
    pub telemetry_context: TelemetryContext,
}

impl SessionCreationContext {
    pub fn new(
        context: Context,
        remote_addr: String,
        user_agent: String,
        capabilities: String,
        telemetry_context: TelemetryContext,
    ) -> Self {
        Self {
            context,
            remote_addr,
            user_agent,
            capabilities,
            telemetry_context,
        }
    }
}

impl Deref for SessionCreationContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
