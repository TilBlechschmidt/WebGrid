use super::super::Context;
use crate::libraries::metrics::MetricsEntry;
use anyhow::Result;
use async_trait::async_trait;
use jatsl::{Job, TaskManager};
use log::{debug, info, warn};
use std::time::Duration;
use tokio::time::sleep;

pub struct CleanupJob {
    size_threshold: f64,
}

#[async_trait]
impl Job for CleanupJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        manager
            .context
            .metrics
            .submit(MetricsEntry::StorageCapacityUpdated(
                manager.context.storage_id.to_string(),
                self.size_threshold,
            ))
            .ok();

        let storage = &manager.context.storage;

        manager.ready().await;

        let mut iteration_counter: u64 = 0;
        let scan_interval = 60 / 5 * 24; // every 24 hours

        loop {
            // Re-enumerate the filesystem to "fix" any drifts that may occur
            if iteration_counter % scan_interval == 0 {
                info!("Synchronising filesystem");
                if let Err(e) = storage.scan_fs().await {
                    warn!("Error while synchronising file system: {}", e);
                }
            }

            // Run the cleanup if the threshold is exceeded
            debug!("Running cleanup cycle #{}", iteration_counter);
            let file_count = storage.maybe_cleanup().await?;

            if file_count > 0 {
                info!("Cleaned up {} files", file_count);
            }

            // Update the storage metrics
            if let Ok(usage) = storage.used_bytes().await {
                manager
                    .context
                    .metrics
                    .submit(MetricsEntry::StorageUsageUpdated(
                        manager.context.storage_id.to_string(),
                        usage,
                    ))
                    .ok();
            }

            // Wait for the next cycle
            sleep(Duration::from_secs(60 * 5)).await;
            iteration_counter += 1;
        }
    }
}

impl CleanupJob {
    pub fn new(size_threshold: f64) -> Self {
        Self { size_threshold }
    }
}
