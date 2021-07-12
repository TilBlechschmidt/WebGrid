use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use crate::harness::{DeathReason, ModuleTerminationReason};
use crate::library::communication::event::{Notification, QueueDescriptor};
use crate::library::communication::BlackboxError;
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "session.terminated";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Reason for a session to commence shutdown
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionTerminationReason {
    /// An error occured during the session startup or job scheduling routine
    StartupFailed(BlackboxError),
    /// Either the startup or shutdown routine timed out
    ModuleTimeout,
    /// Termination was requested by the client
    ClosedByClient(String),
    /// No requests have been received within the idle timeout period
    IdleTimeoutReached,
    /// External process signals terminated the session
    TerminatedExternally,
}

/// Session has terminated and is no longer reachable
///
/// Whenever a session that has previously sent the [`SessionOperationalNotification`](super::SessionOperationalNotification)
/// becomes unreachable permanently due to a particular [reason](SessionTerminationReason), this event is fired.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionTerminatedNotification {
    /// Unique identifier of the created session
    pub id: SessionIdentifier,

    /// Reason for the termination
    pub reason: SessionTerminationReason,
}

impl Notification for SessionTerminatedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}

impl From<ModuleTerminationReason> for SessionTerminationReason {
    fn from(reason: ModuleTerminationReason) -> Self {
        match reason {
            ModuleTerminationReason::StartupFailed(e) => {
                SessionTerminationReason::StartupFailed(BlackboxError::from_boxed(e))
            }
            ModuleTerminationReason::OperationalError(e) => {
                SessionTerminationReason::StartupFailed(BlackboxError::from_boxed(e))
            }
            ModuleTerminationReason::HeartDied(DeathReason::Killed(description)) => {
                SessionTerminationReason::ClosedByClient(description)
            }
            ModuleTerminationReason::HeartDied(DeathReason::LifetimeExceeded) => {
                SessionTerminationReason::IdleTimeoutReached
            }
            ModuleTerminationReason::HeartDied(DeathReason::Terminated) => {
                SessionTerminationReason::TerminatedExternally
            }
            ModuleTerminationReason::Timeout => SessionTerminationReason::ModuleTimeout,
            ModuleTerminationReason::ExitedNormally => unreachable!(),
        }
    }
}
