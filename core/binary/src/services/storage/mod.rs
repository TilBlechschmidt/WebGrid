use anyhow::Result;
use helpers::{constants, SharedOptions};
use lifecycle::Heart;
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};
use std::path::PathBuf;
use storage_lib::StorageHandler;
use structopt::StructOpt;
use uuid::Uuid;

mod context;
mod jobs;

use context::Context;
use jobs::ServerJob;

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
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let storage_id = StorageHandler::storage_id(options.storage_directory.clone()).await?;
    let provider_id = Uuid::new_v4().to_string();

    let (mut heart, _) = Heart::new();

    let context = Context::new(shared_options.redis, storage_id, options.host, options.port);
    let scheduler = JobScheduler::new();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let server_job = ServerJob::new(options.port, options.storage_directory);

    context.spawn_heart_beat(&provider_id, &scheduler).await;

    schedule!(scheduler, context, { status_job, server_job });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
