use super::{Provisioner, ProvisionerType};
use crate::libraries::helpers::keys;
use crate::libraries::lifecycle::HeartBeat;
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::scheduling::JobScheduler;
use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<DefaultResourceManager>,
    pub provisioner: Arc<Box<dyn Provisioner + Send + Sync + 'static>>,
    pub provisioner_type: ProvisionerType,
    pub id: String,
}

impl Context {
    pub fn new<P: Provisioner + Send + Sync + Clone + 'static>(
        provisioner_type: ProvisionerType,
        provisioner: P,
        redis_url: String,
        id: String,
    ) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat: HeartBeat::new(),
            provisioner: Arc::new(Box::new(provisioner)),
            provisioner_type,
            id,
        }
    }

    pub async fn spawn_heart_beat(&self, scheduler: &JobScheduler) {
        self.heart_beat
            .add_beat(&keys::orchestrator::heartbeat(&self.id), 60, 120)
            .await;
        scheduler.spawn_job(self.heart_beat.clone(), self.resource_manager.clone());
    }
}
