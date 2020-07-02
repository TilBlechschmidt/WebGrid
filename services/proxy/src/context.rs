use crate::routing_info::RoutingInfo;
use resources::DefaultResourceManager;

#[derive(Clone)]
pub struct Context {
    pub resource_manager: DefaultResourceManager,
    pub routing_info: RoutingInfo,
}

impl Context {
    pub fn new() -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(),
            routing_info: RoutingInfo::new(),
        }
    }
}
