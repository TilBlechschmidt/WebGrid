use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use crate::library::communication::event::{Notification, QueueDescriptor};
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "session.operational";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Session has finished startup
///
/// This event is fired when a session has completed startup, is now
/// [discoverable](crate::library::communication::discovery), and can
/// handle WebDriver requests.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionOperationalNotification {
    /// Unique identifier of the created session
    pub id: SessionIdentifier,

    /// Raw [`Capabilities`](crate::domain::webdriver::Capabilities) json string
    /// returned by the WebDriver implementation.
    pub actual_capabilities: String,
}

impl Notification for SessionOperationalNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
