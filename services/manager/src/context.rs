use helpers::keys;
use lifecycle::HeartBeat;
use resources::DefaultResourceManager;
use scheduling::JobScheduler;
use std::ops::Deref;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub heart_beat: HeartBeat<DefaultResourceManager>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(),
            heart_beat: HeartBeat::new(),
        }
    }

    pub async fn spawn_heart_beat(&self, scheduler: &JobScheduler) {
        self.heart_beat
            .add_beat(&keys::manager::HEARTBEAT, 60, 120)
            .await;
        scheduler.spawn_job(self.heart_beat.clone(), self.resource_manager.clone());
    }
}

pub struct SessionCreationContext {
    context: Context,
    pub remote_addr: String,
    pub user_agent: String,
    pub capabilities: String,
}

impl SessionCreationContext {
    pub fn new(
        context: Context,
        remote_addr: String,
        user_agent: String,
        capabilities: String,
    ) -> Self {
        Self {
            context,
            remote_addr,
            user_agent,
            capabilities,
        }
    }
}

impl Deref for SessionCreationContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}
