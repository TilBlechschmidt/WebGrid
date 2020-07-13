use crate::{tasks::DriverReference, Options};
use helpers::keys;
use lifecycle::HeartBeat;
use resources::DefaultResourceManager;
use scheduling::JobScheduler;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub driver_reference: DriverReference,
    pub heart_beat: HeartBeat<DefaultResourceManager>,
    pub id: String,
    pub options: Options,
}

impl Context {
    pub fn new(redis_url: String, options: Options) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            driver_reference: DriverReference::new(),
            heart_beat: HeartBeat::new(),
            id: options.id.clone(),
            options,
        }
    }

    pub async fn spawn_heart_beat(&self, scheduler: &JobScheduler) {
        self.heart_beat
            .add_beat(&keys::session::heartbeat::node(&self.id), 60, 120)
            .await;
        scheduler.spawn_job(self.heart_beat.clone(), self.resource_manager.clone());
    }
}
