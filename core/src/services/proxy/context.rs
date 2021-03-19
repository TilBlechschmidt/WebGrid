use super::routing_info::RoutingInfo;
use crate::libraries::resources::{DefaultResourceManager, ResourceManagerProvider};

#[derive(Clone)]
pub struct Context {
    resource_manager: DefaultResourceManager,
    pub routing_info: RoutingInfo,
}

impl Context {
    pub fn new(redis_url: String) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            routing_info: RoutingInfo::new(),
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
