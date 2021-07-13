//! Structures related to processing [`ProvisionerMatchRequests`](crate::domain::request::ProvisionerMatchRequest)

use crate::domain::webdriver::CapabilitiesRequest;

mod container;
mod service;

pub use container::ContainerMatchingStrategy;
pub use service::ProvisionerMatchingService;

/// Decides whether or not a [`CapabilityRequest`](CapabilitiesRequest) can be fulfilled
pub trait MatchingStrategy {
    /// Whether a given request can be fulfilled by the linked provisioner
    fn matches(&self, request: CapabilitiesRequest) -> bool;
}

impl<'a> MatchingStrategy for Box<dyn MatchingStrategy + Send + Sync + 'a> {
    fn matches(&self, request: CapabilitiesRequest) -> bool {
        self.as_ref().matches(request)
    }
}
