use super::super::super::QUEUE_SIZE_STARTUP_WORKFLOW;
use super::SessionIdentifier;
use library::communication::event::{Notification, QueueDescriptor};
use library::communication::BlackboxError;
use library::BoxedError;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Error as FmtError, Formatter},
};
use thiserror::Error;

const QUEUE_KEY: &str = "session.terminated";
const QUEUE_SIZE: usize = QUEUE_SIZE_STARTUP_WORKFLOW;

/// Reason why the sessions heart stopped beating
#[derive(Debug, Clone)]
pub enum DeathReason {
    /// Internal kill signal has been sent
    Killed(String),
    /// Predetermined lifetime has been exceeded
    LifetimeExceeded,
    /// SIGINT or other process-external cause
    Terminated,
}

/// Reason why a module has terminated
#[derive(Error, Debug)]
pub enum ModuleTerminationReason {
    /// Startup routine threw an error
    #[error("startup routine threw an error")]
    StartupFailed(#[source] BoxedError),
    /// Core run loop threw an error
    #[error("error during operation")]
    OperationalError(#[source] BoxedError),
    /// [`Heart`] provided by module died
    #[error("heart provided by module died: {0}")]
    HeartDied(DeathReason),
    /// Run loop exited cleanly
    #[error("run loop exited cleanly")]
    ExitedNormally,
    /// Timeout during startup or shutdown
    #[error("timeout during startup or shutdown")]
    Timeout,
}

/// Reason for a session to commence shutdown
#[derive(Error, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(tag = "reason")]
pub enum SessionTerminationReason {
    /// Session failed to reach an operational state
    ///
    /// Whenever a component partaking in the session startup workflow encounters a
    /// critical failure, this event is triggered. It indicates an unrecoverable error
    /// in the startup sequence and thus the affected session may be considered dead.
    #[error("An error occured during the session startup or job scheduling routine")]
    StartupFailed {
        /// Stacktrace of the error that caused the failure
        #[source]
        error: BlackboxError,
    },
    /// Either the startup or shutdown routine timed out
    #[error("Either the startup or shutdown routine timed out")]
    ModuleTimeout,
    /// Termination was requested by the client
    #[error("Termination was requested by the client ({message})")]
    ClosedByClient {
        /// Description on why or how the client closed the session
        message: String,
    },
    /// No requests have been received within the idle timeout period
    #[error("No requests have been received within the idle timeout period")]
    IdleTimeoutReached,
    /// External process signals terminated the session
    #[error("External process signals terminated the session")]
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

    /// Bytes of video recorded
    pub recording_bytes: usize,
}

impl Notification for SessionTerminatedNotification {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}

impl SessionTerminatedNotification {
    /// Shorthand to create a [`SessionTerminatedNotification`] for a startup failure
    pub fn new_for_startup_failure(id: SessionIdentifier, error: BlackboxError) -> Self {
        Self {
            id,
            reason: SessionTerminationReason::StartupFailed { error },
            recording_bytes: 0,
        }
    }
}

impl From<ModuleTerminationReason> for SessionTerminationReason {
    fn from(reason: ModuleTerminationReason) -> Self {
        match reason {
            ModuleTerminationReason::StartupFailed(e) => SessionTerminationReason::StartupFailed {
                error: BlackboxError::from_boxed(e),
            },
            ModuleTerminationReason::OperationalError(e) => {
                SessionTerminationReason::StartupFailed {
                    error: BlackboxError::from_boxed(e),
                }
            }
            ModuleTerminationReason::HeartDied(DeathReason::Killed(message)) => {
                SessionTerminationReason::ClosedByClient { message }
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

impl fmt::Display for DeathReason {
    fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            DeathReason::Killed(reason) => write!(w, "Killed ({})", reason),
            DeathReason::LifetimeExceeded => write!(w, "Lifetime was exceeded"),
            DeathReason::Terminated => write!(w, "Terminated due to external signal"),
        }
    }
}
