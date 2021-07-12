use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use crate::library::communication::event::{Notification, QueueDescriptor};
use crate::library::communication::BlackboxError;
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "session.failed";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Session failed to reach an operational state
///
/// Whenever a component partaking in the session startup workflow encounters a
/// critical failure, this event is triggered. It indicates an unrecoverable error
/// in the startup sequence and thus the affected session may be considered dead.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStartupFailedNotification {
    /// Unique identifier of the created session
    pub id: SessionIdentifier,

    /// Cause of the startup failure
    pub cause: BlackboxError,
}

impl Notification for SessionStartupFailedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
