use std::collections::HashMap;

use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use library::communication::event::{Notification, QueueDescriptor};
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "session.metadata";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Key-value metadata attached to a session by a client
pub type SessionClientMetadata = HashMap<String, String>;

/// Session metadata has been modified by the client
///
/// This event may be fired whenever a client by some means makes changes to the sessions metadata.
/// Changes can either be submitted in the initial capabilities request or through special REST endpoints.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionMetadataModifiedNotification {
    /// Unique identifier of the created session
    pub id: SessionIdentifier,

    /// Key-value pairs that have been modified by the client
    pub metadata: SessionClientMetadata,
}

impl Notification for SessionMetadataModifiedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
