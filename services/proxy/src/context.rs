use crate::routing_info::RoutingInfo;
use resources::DefaultResourceManager;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
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
