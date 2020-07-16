use helpers::SharedOptions;
use lifecycle::Heart;
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};
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

    schedule!(scheduler, context, { status_job });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
