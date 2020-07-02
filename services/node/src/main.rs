use anyhow::Result;
use lifecycle::service_init;
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};

mod context;
mod jobs;
mod structs;
mod tasks;

use context::Context;
use jobs::ProxyJob;
use tasks::{initialize_service, initialize_session, start_driver, stop_driver, terminate};

async fn launch_session(context: &Context) -> Result<()> {
    let scheduler = JobScheduler::new();

    // TODO Handle error and go straight to cleanup jobs + make a serial-task-execution macro
    let (mut heart, heart_stone) =
        JobScheduler::spawn_task(&initialize_service, context.clone()).await???;
    JobScheduler::spawn_task(&start_driver, context.clone()).await???;
    let internal_session_id =
        JobScheduler::spawn_task(&initialize_session, context.clone()).await???;

    let status_job = StatusServer::new(&scheduler);
    let proxy_job = ProxyJob::new(internal_session_id, heart_stone);

    context.spawn_heart_beat(&scheduler).await;

    schedule!(scheduler, context, {
        status_job,
        proxy_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    // TODO Send STIMEOUT || CLOSED log status code depending on the death reason

    scheduler.terminate_jobs().await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    service_init();

    let context = Context::new();

    launch_session(&context).await.ok();

    JobScheduler::spawn_task(&terminate, context.clone()).await???;
    JobScheduler::spawn_task(&stop_driver, context).await???;

    // TODO Send HALT log status code

    Ok(())
}
