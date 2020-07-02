use lifecycle::{service_init, Heart};
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};

mod context;
mod jobs;
mod routing_info;

use context::Context;
use jobs::{ProxyJob, WatcherJob};

#[tokio::main]
async fn main() {
    service_init();

    let (mut heart, _) = Heart::new();

    let context = Context::new();
    let scheduler = JobScheduler::new();

    let status_job = StatusServer::new(&scheduler);
    let watcher_job = WatcherJob::new();
    let proxy_job = ProxyJob::new();

    schedule!(scheduler, context, {
        status_job,
        watcher_job,
        proxy_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
