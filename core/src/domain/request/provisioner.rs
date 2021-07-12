use super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use crate::domain::event::ProvisionerIdentifier;
use crate::domain::webdriver::CapabilitiesRequest;
use crate::library::communication::event::{Notification, QueueDescriptor};
use crate::library::communication::request::{Request, ResponseLocation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const QUEUE_KEY: &str = "provisioner.match";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Request to read an object from persistent storage
#[derive(Debug, Serialize, Deserialize, Eq)]
pub struct ProvisionerMatchRequest {
    /// Capabilities object a responder has to support
    pub capabilities: CapabilitiesRequest,

    response_location: ResponseLocation,
}

impl ProvisionerMatchRequest {
    /// Creates a new instance from capabilities and a randomly assigned response location
    pub fn new(capabilities: CapabilitiesRequest) -> Self {
        Self {
            capabilities,
            response_location: Uuid::new_v4().to_string(),
        }
    }
}

impl PartialEq for ProvisionerMatchRequest {
    fn eq(&self, other: &Self) -> bool {
        self.capabilities == other.capabilities
    }
}

/// Response to a [`ProvisionerMatchRequest`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisionerMatchResponse {
    /// Unique identifier of the matched provisioner
    /// capable of supporting the requested capabilities
    pub provisioner: ProvisionerIdentifier,
    // TODO Add load factor
}

impl Notification for ProvisionerMatchRequest {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}

impl Request for ProvisionerMatchRequest {
    type Response = ProvisionerMatchResponse;

    fn reply_to(&self) -> ResponseLocation {
        // TODO This clone feels unnecessary ...
        self.response_location.clone()
    }
}
