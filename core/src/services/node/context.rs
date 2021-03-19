use super::{tasks::DriverReference, Options};
use crate::libraries::lifecycle::HeartBeat;
use crate::libraries::resources::DefaultResourceManager;
use crate::libraries::{helpers::keys, resources::ResourceManagerProvider};

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub driver_reference: DriverReference,
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub id: String,
    pub options: Options,
}

impl Context {
    pub async fn new(redis_url: String, options: Options) -> Self {
        let id = options.id.clone();
        let heart_beat = HeartBeat::new();

        heart_beat
            .add_beat(&keys::session::heartbeat::node(&id), 60, 120)
            .await;

        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            driver_reference: DriverReference::new(),
            heart_beat,
            id,
            options,
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
