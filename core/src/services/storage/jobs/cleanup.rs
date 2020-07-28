use super::super::Context;
use crate::libraries::scheduling::{Job, TaskManager};
use crate::libraries::storage::StorageHandler;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, info, warn};
use std::{path::PathBuf, time::Duration};
use tokio::time::delay_for;

#[derive(Clone)]
pub struct CleanupJob {
    storage_directory: PathBuf,
    size_threshold: f64,
    cleanup_target: f64,
}

#[async_trait]
impl Job for CleanupJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let storage = StorageHandler::new(
            self.storage_directory.clone(),
            self.size_threshold,
            self.cleanup_target,
        )
        .await?;

        manager.ready().await;

        let mut iteration_counter: u64 = 0;
        let scan_interval = 60 / 5 * 24; // every 24 hours

        loop {
            if iteration_counter % scan_interval == 0 {
                info!("Synchronising filesystem");
                if let Err(e) = storage.scan_fs().await {
                    warn!("Error while synchronising file system: {}", e);
                }
            }

            debug!("Running cleanup cycle #{}", iteration_counter);
            let file_count = storage.maybe_cleanup().await?;

            if file_count > 0 {
                info!("Cleaned up {} files", file_count);
            }

            delay_for(Duration::from_secs(60 * 5)).await;
            iteration_counter += 1;
        }
    }
}

impl CleanupJob {
    pub fn new(storage_directory: PathBuf, size_threshold: f64, cleanup_target: f64) -> Self {
        debug!("Size threshold: {} bytes", size_threshold);
        debug!("Cleanup target: {} bytes", cleanup_target);
        Self {
            storage_directory,
            size_threshold,
            cleanup_target,
        }
    }
}
