use crate::tasks::DriverReference;
use helpers::{env, keys};
use lifecycle::HeartBeat;
use resources::DefaultResourceManager;
use scheduling::JobScheduler;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub driver_reference: DriverReference,
    pub heart_beat: HeartBeat<DefaultResourceManager>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(),
            driver_reference: DriverReference::new(),
            heart_beat: HeartBeat::new(),
        }
    }

    pub async fn spawn_heart_beat(&self, scheduler: &JobScheduler) {
        self.heart_beat
            .add_beat(
                &keys::session::heartbeat::node(&env::service::node::ID),
                60,
                120,
            )
            .await;
        scheduler.spawn_job(self.heart_beat.clone(), self.resource_manager.clone());
    }
}
