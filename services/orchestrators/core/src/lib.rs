use lifecycle::{service_init, Heart};
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};

mod context;
mod jobs;
pub mod provisioner;

use context::Context;
use jobs::*;
use provisioner::{Provisioner, Type as ProvisionerType};

pub async fn start<P: Provisioner + Send + Sync + Clone + 'static>(
    provisioner_type: ProvisionerType,
    provisioner: P,
) {
    service_init();

    let (mut heart, _) = Heart::new();

    let context = Context::new(provisioner_type, provisioner);
    let scheduler = JobScheduler::new();

    context.spawn_heart_beat(&scheduler).await;

    let status_job = StatusServer::new(&scheduler);
    let registration_job = RegistrationJob::new();
    let node_watcher_job = NodeWatcherJob::new();
    let slot_reclaim_job = SlotReclaimJob::new();
    let slot_recycle_job = SlotRecycleJob::new();
    let processor_job = ProcessorJob::new();
    let slot_count_adjuster_job = SlotCountAdjusterJob::new();

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
