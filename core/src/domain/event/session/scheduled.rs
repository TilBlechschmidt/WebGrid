use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::super::ProvisionerIdentifier;
use super::SessionIdentifier;
use crate::library::communication::event::{Notification, QueueDescriptor};
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "session.scheduled";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Session has been assigned to a provisioner
///
/// This event is fired once a session has been matched against all available provisioners
/// and statically assigned to one.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionScheduledNotification {
    /// Unique identifier of the scheduled session
    pub id: SessionIdentifier,

    /// Identifier of the provisioner the session has been assigned to
    pub provisioner: ProvisionerIdentifier,
}

impl Notification for SessionScheduledNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
