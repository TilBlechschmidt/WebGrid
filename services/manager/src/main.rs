use lifecycle::{service_init, Heart};
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};

mod context;
mod jobs;
mod structures;
mod tasks;

use context::Context;
use jobs::{RegistrationJob, SessionHandlerJob};
pub use structures::*;

#[tokio::main]
async fn main() {
    service_init();

    let (mut heart, _) = Heart::new();

    let context = Context::new();
    let scheduler = JobScheduler::new();

    context.spawn_heart_beat(&scheduler).await;

    let status_job = StatusServer::new(&scheduler);
    let session_handler_job = SessionHandlerJob::new();
    let registration_job = RegistrationJob::new();

    schedule!(scheduler, context, {
        status_job,
        session_handler_job
        registration_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
