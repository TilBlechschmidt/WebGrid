use super::{tasks::DriverReference, Options};
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};
use crate::libraries::{lifecycle::HeartBeat, recording::SequentialWebVttWriter};
use opentelemetry::Context as TelemetryContext;
use std::ops::Deref;
use std::sync::Arc;
use tokio::{fs::File, sync::Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub driver_reference: DriverReference,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub id: Uuid,
    pub options: Options,
    pub webvtt: Arc<Mutex<Option<SequentialWebVttWriter<File>>>>,
}

impl Context {
    pub async fn new(redis_url: String, options: Options) -> Self {
        let id = options.id;
        let heart_beat = HeartBeat::new();

        heart_beat
            .add_beat(&keys::session::heartbeat::node(&id.to_string()), 60, 120)
            .await;

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            driver_reference: DriverReference::new(),
            heart_beat,
            id,
            options,
            webvtt: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_telemetry_context(self, telemetry_context: TelemetryContext) -> StartupContext {
        StartupContext::new(self, telemetry_context)
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}

#[derive(Clone)]
pub struct StartupContext {
    context: Context,
    pub telemetry_context: TelemetryContext,
}

impl StartupContext {
    fn new(context: Context, telemetry_context: TelemetryContext) -> Self {
        Self {
            context,
            telemetry_context,
        }
    }
}

impl Deref for StartupContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
