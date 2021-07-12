use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use crate::library::communication::event::{Notification, QueueDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const QUEUE_KEY: &str = "session.provisioned";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Additional information provided by the provisioner
///
/// This may include contextual information about the deployment
/// like the Kubernetes namespace, container labels, or the hostname
/// of the machine the session will be running on.
///
// TODO Currently, the keys and values are not strongly typed.
//      Might be worth a consideration in the future.
pub type ProvisionedSessionMetadata = HashMap<String, String>;

/// Session has been provisioned
///
/// Fired when the provisioner service has received confirmation from the
/// underlying infrastructure (e.g. Kubernetes) that a node process will
/// be created.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionProvisionedNotification {
    /// Unique identifier of the provisioned session
    pub id: SessionIdentifier,

    /// Additional metadata, see [`ProvisionedSessionMetadata`]
    pub meta: ProvisionedSessionMetadata,
}

impl Notification for SessionProvisionedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
