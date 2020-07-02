use crate::{Provisioner, ProvisionerType};
use helpers::keys;
use lifecycle::HeartBeat;
use resources::DefaultResourceManager;
use scheduling::JobScheduler;
use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<DefaultResourceManager>,
    pub provisioner: Arc<Box<dyn Provisioner + Send + Sync + 'static>>,
    pub provisioner_type: ProvisionerType,
}

impl Context {
    pub fn new<P: Provisioner + Send + Sync + Clone + 'static>(
        provisioner_type: ProvisionerType,
        provisioner: P,
    ) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(),
            heart_beat: HeartBeat::new(),
            provisioner: Arc::new(Box::new(provisioner)),
            provisioner_type,
        }
    }

    pub async fn spawn_heart_beat(&self, scheduler: &JobScheduler) {
        self.heart_beat
            .add_beat(&keys::orchestrator::HEARTBEAT, 60, 120)
            .await;
        scheduler.spawn_job(self.heart_beat.clone(), self.resource_manager.clone());
    }
}
