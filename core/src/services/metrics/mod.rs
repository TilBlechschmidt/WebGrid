//! Prometheus metric provider

use super::SharedOptions;
use crate::libraries::helpers::constants;
use crate::libraries::lifecycle::Heart;
use crate::libraries::scheduling::{JobScheduler, StatusServer};
use crate::schedule;
use context::Context;
use jobs::MetricHandlerJob;
use log::info;
use structopt::StructOpt;

mod context;
mod data_collector;
mod jobs;
mod structures;

#[derive(Debug, StructOpt)]
/// Prometheus metric provider
///
/// Uses lifecycle and health probes from all active grid components to provide metrics for Prometheus.
pub struct Options {
    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_METRICS)]
    port: u16,
}

pub async fn run(shared_options: SharedOptions, options: Options) {
    let scheduler = JobScheduler::default();
    let (mut heart, _) = Heart::new();
    let context = Context::new(shared_options.redis);

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let metrics_job = MetricHandlerJob::new(options.port);

    schedule!(scheduler, context, {
        status_job,
        metrics_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
