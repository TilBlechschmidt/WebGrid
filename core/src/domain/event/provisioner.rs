use super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use crate::domain::webdriver::RawCapabilitiesRequest;
use crate::library::communication::event::{Notification, QueueDescriptor};
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "provisioner.job.assigned";
/// Note that this value effectively determines how many sessions may be queued
/// and if the size is ever reached without queued items being processed, they
/// will get discarded.
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW * 2;

/// Unique identifier of a provisioner
pub type ProvisionerIdentifier = String;

/// Provisioner has been assigned a specific job
///
/// This event is usually derived from the [`SessionScheduledNotification`](super::session::SessionScheduledNotification)
/// and provides an indirection so that a more accurate queue length can be
/// determined. Additionally, it reduces the load on the provisioner as it
/// no longer has to filter through all notifications (even those meant for others).
///
/// It is intended to be used with a [`QueueDescriptorExtension`](crate::library::communication::event::QueueDescriptorExtension)
/// containing the provisioner identifier.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisioningJobAssignedNotification {
    /// Unique identifier of the scheduled session
    pub session_id: SessionIdentifier,

    /// Raw [`CapabilitiesRequest`](crate::domain::webdriver::CapabilitiesRequest) json string used for scheduling.
    pub capabilities: RawCapabilitiesRequest,
}

impl Notification for ProvisioningJobAssignedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}
