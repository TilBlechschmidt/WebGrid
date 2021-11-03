use crate::library::BoxedError;

use super::super::library::EmptyResult;
use super::{DeathReason, Heart};
use async_trait::async_trait;
use jatsl::{JobScheduler, StatusServer};
use std::any::type_name;
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument};

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
    #[instrument(skip(self))]
    async fn post_shutdown(&mut self, termination_reason: ModuleTerminationReason) {
        match termination_reason {
            ModuleTerminationReason::HeartDied(_) | ModuleTerminationReason::ExitedNormally => {
                info!("Module exited normally")
            }
            _ => error!("Module terminated with an error"),
        }
    }
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
    #[instrument(skip(self, module), fields(module_name = type_name::<M>()))]
    pub async fn run<M: Module + Send + Sync>(&self, mut module: M) {
        let scheduler = JobScheduler::default();
        let mut termination_reason = ModuleTerminationReason::ExitedNormally;

        if let Some(port) = self.status_server_port {
            info!(port, "Spawning status server");
            // TODO Provide the status server with a "fake job" that is not ready while we the module.run() method has not been awaited
            let status_server = StatusServer::new(&scheduler, port);
            scheduler.spawn_job(status_server).await;
        }

        info!("Commencing module startup sequence");
        let startup = timeout(self.startup_timeout, module.pre_startup()).await;

        match startup {
            Ok(Ok(_)) => {
                self.run_loop(&mut module, &scheduler, &mut termination_reason)
                    .await
            }
            Ok(Err(error)) => {
                error!(?error, "Module startup sequence encountered an error");
                termination_reason = ModuleTerminationReason::StartupFailed(error);
            }
            Err(_) => {
                error!("Module startup sequence timed out");
                termination_reason = ModuleTerminationReason::Timeout
            }
        }

        info!("Terminating remaining jobs");
        scheduler.terminate_jobs().await;

        info!("Commencing module shutdown sequence");
        let result = timeout(
            self.shutdown_timeout,
            module.post_shutdown(termination_reason),
        )
        .await;

        if result.is_err() {
            error!("Module shutdown sequence timed out");
        }
    }

    #[instrument(skip(self, module, scheduler, termination_reason))]
    async fn run_loop<M: Module + Send + Sync>(
        &self,
        module: &mut M,
        scheduler: &JobScheduler,
        termination_reason: &mut ModuleTerminationReason,
    ) {
        info!("Executing module run procedure");
        match module.run(scheduler).await {
            Ok(None) => {
                debug!("Module run procedure completed successfully");
            }
            Ok(Some(mut heart)) => {
                debug!("Module run procedure completed successfully, entering run loop");
                let death_reason = heart.death().await;
                info!(?death_reason, "Heart provided by run procedure died");
                *termination_reason = ModuleTerminationReason::HeartDied(death_reason);
            }
            Err(error) => {
                info!(?error, "Module run procedure encountered an error");
                *termination_reason = ModuleTerminationReason::OperationalError(error);
            }
        }
    }
}
