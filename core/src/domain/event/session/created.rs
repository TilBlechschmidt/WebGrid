use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use crate::domain::webdriver::RawCapabilitiesRequest;
use crate::library::communication::event::{Notification, QueueDescriptor};
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "session.created";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Session has been created by a client
///
/// This event may be fired if a client by some means requested the creation of a new session.
/// It contains all the information provided by the client and in addition a generated UUID.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCreatedNotification {
    /// Unique identifier of the created session
    pub id: SessionIdentifier,

    /// Raw [`CapabilitiesRequest`](crate::domain::webdriver::CapabilitiesRequest) json string provided by the end-user
    pub capabilities: RawCapabilitiesRequest,
}

impl Notification for SessionCreatedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
