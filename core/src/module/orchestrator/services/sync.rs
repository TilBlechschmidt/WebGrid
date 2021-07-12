use crate::library::EmptyResult;

use super::{super::provisioner::SessionProvisioner, ProvisioningState};
use async_trait::async_trait;
use jatsl::{Job, JobManager};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Ensures the internal managed session list is in agreement with the hardware
pub struct HardwareSynchronisationService<S: SessionProvisioner> {
    state: ProvisioningState,
    provisioner: Arc<S>,
    interval: Duration,
}

impl<S: SessionProvisioner> HardwareSynchronisationService<S> {
    pub fn new(state: ProvisioningState, provisioner: Arc<S>, interval: Duration) -> Self {
        Self {
            state,
            provisioner,
            interval,
        }
    }
}

#[async_trait]
impl<S> Job for HardwareSynchronisationService<S>
where
    S: SessionProvisioner + Send + Sync,
{
    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: JobManager) -> EmptyResult {
        manager.ready().await;

        // TODO Switch this up so it either waits until a timeout is reached or a number of session.terminated notifications have been received
        //      This allows high-throughput grids to release the resources earlier :)

        loop {
            sleep(self.interval).await;

            match self.provisioner.alive_sessions().await {
                Ok(alive_sessions) => self.state.release_dead_sessions(alive_sessions).await,
                Err(e) => log::warn!("Failed to fetch alive sessions from provisioner: {}", e),
            }
        }
    }
}
