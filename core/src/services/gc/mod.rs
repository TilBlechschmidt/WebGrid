//! Garbage collector service

use super::SharedOptions;
use crate::libraries::lifecycle::Heart;
use crate::libraries::scheduling::{JobScheduler, StatusServer};
use crate::schedule;
use anyhow::Result;
use log::info;
use structopt::StructOpt;

mod context;
mod jobs;

use context::Context;
use jobs::GarbageCollectorJob;

#[derive(Debug, StructOpt)]
/// Garbage collector service
///
/// Purges old or orphaned data from the database.
pub struct Options {
    /// Duration in seconds to retain a terminated session's metadata
    #[structopt(short, long, env, default_value = "604800")]
    session_retention_duration: i64,
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let (mut heart, _) = Heart::new();

    let context = Context::new(shared_options.redis);
    let scheduler = JobScheduler::default();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let gc_job = GarbageCollectorJob::new(options.session_retention_duration);

    schedule!(scheduler, context, {
        status_job,
        gc_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
