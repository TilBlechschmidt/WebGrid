use helpers::SharedOptions;
use lifecycle::Heart;
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};
use structopt::StructOpt;

mod context;
mod jobs;
pub mod provisioner;

use context::Context;
use jobs::*;
use provisioner::{Provisioner, Type as ProvisionerType};

#[derive(Debug, StructOpt)]
/// Provisioner for new session nodes
///
/// Different implementations for e.g. local, Docker or Kubernetes are available.
pub struct Options {
    /// Unique instance identifier
    ///
    /// Used to recover previously allocated slots after a restart.
    #[structopt(env)]
    id: String,

    /// Number of concurrent sessions
    #[structopt(long, env)]
    slot_count: usize,
}

pub async fn start<P: Provisioner + Send + Sync + Clone + 'static>(
    provisioner_type: ProvisionerType,
    provisioner: P,
    options: Options,
    shared_options: SharedOptions,
) {
    let (mut heart, _) = Heart::new();

    let context = Context::new(
        provisioner_type,
        provisioner,
        shared_options.redis,
        options.id,
    );
    let scheduler = JobScheduler::new();

    context.spawn_heart_beat(&scheduler).await;

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let registration_job = RegistrationJob::new();
    let node_watcher_job = NodeWatcherJob::new();
    let slot_reclaim_job = SlotReclaimJob::new();
    let slot_recycle_job = SlotRecycleJob::new();
    let processor_job = ProcessorJob::new();
    let slot_count_adjuster_job = SlotCountAdjusterJob::new(options.slot_count);

    schedule!(scheduler, context, {
        status_job,
        registration_job
        node_watcher_job
        processor_job
        slot_count_adjuster_job
        slot_reclaim_job
        slot_recycle_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
