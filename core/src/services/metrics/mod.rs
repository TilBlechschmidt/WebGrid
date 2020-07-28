use super::SharedOptions;
use crate::libraries::helpers::constants::PORT_METRICS;
use crate::libraries::lifecycle::Heart;
use crate::libraries::scheduling::{JobScheduler, StatusServer};
use crate::schedule;
use log::info;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
/// Prometheus metric provider
///
/// Uses lifecycle and health probes from all active grid components to provide metrics for Prometheus.
pub struct Options {}

#[derive(Clone)]
struct DummyContext {}

pub async fn run(shared_options: SharedOptions, _options: Options) {
    let scheduler = JobScheduler::new();
    let (mut heart, _) = Heart::new();
    let context = DummyContext {};

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);

    println!("This should listen on {}", PORT_METRICS);

    schedule!(scheduler, context, { status_job });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
