use super::{Provisioner, ProvisionerType};
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};
use crate::libraries::{lifecycle::HeartBeat, net::discovery::ServiceDiscovery};
use opentelemetry::Context as TelemetryContext;
use std::sync::Arc;
use std::{ops::Deref, time::Duration};

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub provisioner: Arc<Box<dyn Provisioner + Send + Sync + 'static>>,
    pub provisioner_type: ProvisionerType,
    pub timeout_startup: Duration,
    pub id: String,
}

impl Context {
    pub async fn new<P: Provisioner + Send + Sync + Clone + 'static>(
        provisioner_type: ProvisionerType,
        provisioner: P,
        redis_url: String,
        timeout_startup: Duration,
        id: String,
    ) -> Self {
        let heart_beat = HeartBeat::new();

        heart_beat
            .add_beat(&keys::orchestrator::heartbeat(&id), 60, 120)
            .await;

        heart_beat
            .add_beat(&keys::orchestrator::retain(&id), 300, 604_800)
            .await;

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat,
            provisioner: Arc::new(Box::new(provisioner)),
            provisioner_type,
            id,
            timeout_startup,
        }
    }

    pub fn into_provisioning_context(
        self,
        session_id: String,
        discovery: ServiceDiscovery,
        telemetry_context: TelemetryContext,
    ) -> ProvisioningContext {
        ProvisioningContext::new(self, session_id, discovery, telemetry_context)
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}

#[derive(Clone)]
pub struct ProvisioningContext {
    context: Context,
    pub discovery: ServiceDiscovery,
    pub session_id: String,
    pub telemetry_context: TelemetryContext,
}

impl ProvisioningContext {
    fn new(
        context: Context,
        session_id: String,
        discovery: ServiceDiscovery,
        telemetry_context: TelemetryContext,
    ) -> Self {
        Self {
            context,
            discovery,
            session_id,
            telemetry_context,
        }
    }
}

impl Deref for ProvisioningContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
