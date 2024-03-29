use super::{super::provisioner::SessionProvisioner, ProvisioningState};
use async_trait::async_trait;
use jatsl::{Job, JobManager};
use library::EmptyResult;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

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

            self.provisioner.purge_terminated().await?;

            let alive_sessions = self.provisioner.alive_sessions().await?;
            let count = alive_sessions.len();
            self.state.release_dead_sessions(alive_sessions).await;

            debug!(
                alive_count = count,
                "Executed hardware synchronization cycle"
            );
        }
    }
}
