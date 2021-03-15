//! Defaults and database accessors for timeout values

use log::{info, trace};
use redis::{aio::ConnectionLike, AsyncCommands};
use std::fmt;

/// Timeout value accessors in seconds
#[derive(Debug)]
pub enum Timeout {
    /// How long a session creation request may wait for an orchestrator
    Queue,
    /// Maximum duration a session may take to be scheduled by an orchestrator
    Scheduling,
    /// How long a session may take to start up
    ///
    /// Note that this can include the time it takes for a provisioner like Kubernetes to assign a pod!
    /// If this timeout is hit it might indicate a scheduling problem in your cluster.
    NodeStartup,
    /// How long the WebDriver executable may take to become responsive
    DriverStartup,
    /// Maximum idle duration of a session
    SessionTermination,
    /// Interval at which orphaned slots are reclaimed
    SlotReclaimInterval,
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Timeout {
    /// Default timeout value if nothing is set in the database
    fn default(&self) -> usize {
        match *self {
            // Manager
            Timeout::Queue => 600,
            Timeout::Scheduling => 60,
            Timeout::NodeStartup => 120,
            // Node
            Timeout::DriverStartup => 30,
            Timeout::SessionTermination => 900,
            // Orchestrator
            Timeout::SlotReclaimInterval => 300,
        }
    }

    /// Retrieve either a value set in the database or initializes it to the default
    pub async fn get<C: ConnectionLike + AsyncCommands>(&self, con: &mut C) -> usize {
        let key = format!("{}", self).to_lowercase();

        trace!("Reading timeout {}", key);
        let timeout: Option<usize> = con.hget("timeouts", &key).await.ok();

        match timeout {
            Some(timeout) => timeout,
            None => {
                info!("Initializing timeout {} to default value", key);
                let default = self.default();
                let _: Option<()> = con.hset("timeouts", key, default).await.ok();
                default
            }
        }
    }
}
