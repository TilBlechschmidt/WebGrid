use crate::library::BoxedError;

use super::super::library::EmptyResult;
use super::{DeathReason, Heart};
use async_trait::async_trait;
use jatsl::{JobScheduler, StatusServer};
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;

/// Executable module
#[async_trait]
pub trait Module {
    /// Executed before running the core loop
    async fn pre_startup(&mut self) -> EmptyResult {
        Ok(())
    }

    /// Core run loop of the module
    ///
    /// When the function returns `Some(_)` the death of the returned [`Heart`] is awaited before calling the shutdown hook.
    /// Useful for situations where you dispatch background jobs in the run loop but want to hand-off the program lifecycle management.
    ///
    /// Returning `None` results in the program entering a shutdown state and calling the `pre_shutdown` hook.
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError>;

    /// Shutdown hook executed after the core loop and all associated jobs have terminated
    async fn post_shutdown(&mut self, _termination_reason: ModuleTerminationReason) {}
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

/// Runner for [`Module`] implementations
pub struct ModuleRunner {
    startup_timeout: Duration,
    shutdown_timeout: Duration,
    status_server_port: Option<u16>,
}

impl ModuleRunner {
    /// Creates a new instance using default timeouts and enabling the status server
    pub fn new_with_status_server(status_server_port: u16) -> Self {
        Self {
            status_server_port: Some(status_server_port),
            ..Default::default()
        }
    }
}

impl Default for ModuleRunner {
    fn default() -> Self {
        Self {
            startup_timeout: Duration::from_secs(60),
            shutdown_timeout: Duration::from_secs(60),
            status_server_port: None,
        }
    }
}

impl ModuleRunner {
    /// Executes a [`Module`] until it exits by calling the corresponding lifecycle functions in order
    /// and returns the reason why it terminated.
    pub async fn run<M: Module + Send + Sync>(&self, mut module: M) {
        let scheduler = JobScheduler::default();
        let mut termination_reason = ModuleTerminationReason::ExitedNormally;

        if let Some(port) = self.status_server_port {
            // TODO Provide the status server with a "fake job" that is not ready while we the module.run() method has not been awaited
            let status_server = StatusServer::new(&scheduler, port);
            scheduler.spawn_job(status_server);
        }

        let startup = timeout(self.startup_timeout, module.pre_startup()).await;

        match startup {
            Ok(Ok(_)) => {
                self.run_loop(&mut module, &scheduler, &mut termination_reason)
                    .await
            }
            Ok(Err(e)) => {
                log::error!("Module startup failed: {}!", e);
                termination_reason = ModuleTerminationReason::StartupFailed(e)
            }
            Err(e) => {
                log::error!("Module startup timed out: {}!", e);
                termination_reason = ModuleTerminationReason::Timeout
            }
        }

        scheduler.terminate_jobs().await;

        let shutdown = timeout(
            self.shutdown_timeout,
            module.post_shutdown(termination_reason),
        )
        .await;

        match shutdown {
            Ok(_) => (),
            Err(e) => {
                log::error!("Module shutdown routine timed out: {}!", e)
            }
        }
    }

    async fn run_loop<M: Module + Send + Sync>(
        &self,
        module: &mut M,
        scheduler: &JobScheduler,
        termination_reason: &mut ModuleTerminationReason,
    ) {
        match module.run(scheduler).await {
            Ok(None) => {}
            Ok(Some(mut heart)) => {
                let death_reason = heart.death().await;
                *termination_reason = ModuleTerminationReason::HeartDied(death_reason);
            }
            Err(e) => {
                *termination_reason = ModuleTerminationReason::OperationalError(e);
            }
        }
    }
}
