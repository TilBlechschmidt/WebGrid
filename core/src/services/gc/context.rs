use crate::libraries::resources::DefaultResourceManager;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
}

impl Context {
    pub fn new(redis_url: String) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
        }
    }
}
