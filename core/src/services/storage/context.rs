use crate::libraries::helpers::keys;
use crate::libraries::lifecycle::{BeatValue, HeartBeat};
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::scheduling::JobScheduler;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<DefaultResourceManager>,
    pub storage_id: String,
}

impl Context {
    pub fn new(redis_url: String, storage_id: String, host: String, port: u16) -> Self {
        let addr = format!("{}:{}", host, port);

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            heart_beat: HeartBeat::with_value(BeatValue::Constant(addr)),
            storage_id,
        }
    }

    pub async fn spawn_heart_beat(&self, provider_id: &str, scheduler: &JobScheduler) {
        self.heart_beat
            .add_beat(&keys::storage::host(&self.storage_id, provider_id), 60, 120)
            .await;
        scheduler.spawn_job(self.heart_beat.clone(), self.resource_manager.clone());
    }
}
