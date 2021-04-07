//! Unified grid entrypoint provider

use super::SharedOptions;
use crate::libraries::helpers::constants;
use crate::libraries::lifecycle::Heart;
use jatsl::{schedule, JobScheduler, StatusServer};
use log::info;
use structopt::StructOpt;

mod context;
mod jobs;
mod routing_info;

use context::Context;
use jobs::{ProxyJob, WatcherJob};

#[derive(Debug, StructOpt)]
/// Unified grid entrypoint provider
///
/// Provides a unified endpoint which routes traffic to available managers or requested sessions.
pub struct Options {
    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_PROXY)]
    port: u16,
}

pub async fn run(shared_options: SharedOptions, options: Options) {
    let (mut heart, _) = Heart::new();

    let context = Context::new(shared_options.redis);
    let scheduler = JobScheduler::default();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let metrics_job = context.metrics.clone();
    let watcher_job = WatcherJob::new();
    let proxy_job = ProxyJob::new(options.port);

    schedule!(scheduler, context, {
        status_job,
        metrics_job,
        watcher_job,
        proxy_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
