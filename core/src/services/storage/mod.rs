//! Content delivery service

use super::SharedOptions;
use crate::libraries::lifecycle::Heart;
use crate::libraries::storage::StorageHandler;
use crate::libraries::{
    helpers::constants,
    net::{advertise::ServiceAdvertisorJob, discovery::ServiceDescriptor},
};
use anyhow::Result;
use jatsl::{schedule, JobScheduler, StatusServer};
use log::{debug, info};
use std::path::PathBuf;
use structopt::StructOpt;

mod context;
mod jobs;

use context::Context;
use jobs::{CleanupJob, MetadataJob, ServerJob};

#[derive(Debug, StructOpt)]
/// Content delivery service
///
/// Serves files located on disk, written by sessions e.g. video recordings
pub struct Options {
    /// Host under which the storage server is reachable by the proxy
    #[structopt(long, env)]
    host: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_STORAGE)]
    port: u16,

    /// Directory to serve
    #[structopt(long, env, parse(from_os_str))]
    storage_directory: PathBuf,

    /// Directory size limit in GB
    #[structopt(long, env)]
    size_limit: f64,

    /// Percentage (0-100) of size limit to purge during the cleanup routine
    #[structopt(long, env, default_value = "20")]
    cleanup_percentage: f64,
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let storage_id = StorageHandler::storage_id(&options.storage_directory).await?;
    let size_threshold = options.size_limit * 1_000_000_000.0;
    let cleanup_target = size_threshold * (options.cleanup_percentage / 100.0);
    let endpoint = format!("{}:{}", options.host, options.port);

    debug!("Size threshold: {} bytes", size_threshold);
    debug!("Cleanup target: {} bytes", cleanup_target);

    let (mut heart, _) = Heart::new();

    let scheduler = JobScheduler::default();
    let context = Context::new(
        &options,
        shared_options.redis,
        storage_id,
        size_threshold,
        cleanup_target,
    )
    .await?;

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let metrics_job = context.metrics.clone();
    let server_job = ServerJob::new(options.port, options.storage_directory.clone());
    let cleanup_job = CleanupJob::new(size_threshold);
    let metadata_job = MetadataJob::new();
    let advertise_job = ServiceAdvertisorJob::new(ServiceDescriptor::Storage(storage_id), endpoint);

    schedule!(scheduler, context, {
        status_job,
        metrics_job,
        server_job,
        cleanup_job,
        metadata_job,
        advertise_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
