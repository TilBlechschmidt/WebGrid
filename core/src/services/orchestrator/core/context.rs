use super::{Provisioner, ProvisionerType};
use crate::libraries::lifecycle::HeartBeat;
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};
use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub provisioner: Arc<Box<dyn Provisioner + Send + Sync + 'static>>,
    pub provisioner_type: ProvisionerType,
    pub id: String,
}

impl Context {
    pub async fn new<P: Provisioner + Send + Sync + Clone + 'static>(
        provisioner_type: ProvisionerType,
        provisioner: P,
        redis_url: String,
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
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
