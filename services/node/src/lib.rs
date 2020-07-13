use anyhow::Result;
use helpers::{constants, SharedOptions};
use log::info;
use scheduling::{schedule, JobScheduler, StatusServer};
use std::path::PathBuf;
use structopt::StructOpt;

mod context;
mod jobs;
mod structs;
mod tasks;

use context::Context;
use jobs::ProxyJob;
use tasks::{initialize_service, initialize_session, start_driver, stop_driver, terminate};

#[derive(Debug, StructOpt, Clone)]
/// Session provider
///
/// Manages the lifecycle of one session in terms of interaction with the driver and screen recording.
/// Bound to the lifecycle of one session and usually not started explicitly but through a provisioner like Docker or Kubernetes.
pub struct Options {
    /// Unique instance identifier
    #[structopt(env)]
    id: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_NODE)]
    port: u16,

    /// Path to WebDriver executable
    #[structopt(short, long, env, parse(from_os_str))]
    driver: PathBuf,

    /// Port on which the driver is listening by default
    #[structopt(long, env)]
    driver_port: u16,

    /// Type of browser
    ///
    /// Internally used to provide workarounds for driver specific bugs
    #[structopt(long, env)]
    browser: String,

    /// Script to execute when browser session has been created
    #[structopt(long, env)]
    on_session_create: Option<String>,
}

async fn launch_session(
    shared_options: SharedOptions,
    options: Options,
    context: &Context,
) -> Result<()> {
    let scheduler = JobScheduler::new();

    // TODO Handle error and go straight to cleanup jobs + make a serial-task-execution macro
    let (mut heart, heart_stone) =
        JobScheduler::spawn_task(&initialize_service, context.clone()).await???;
    JobScheduler::spawn_task(&start_driver, context.clone()).await???;
    let internal_session_id =
        JobScheduler::spawn_task(&initialize_session, context.clone()).await???;

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let proxy_job = ProxyJob::new(options.port, internal_session_id, heart_stone);

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

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let context = Context::new(shared_options.redis.clone(), options.clone());

    launch_session(shared_options, options, &context).await.ok();

    JobScheduler::spawn_task(&terminate, context.clone()).await???;
    JobScheduler::spawn_task(&stop_driver, context).await???;

    // TODO Send HALT log status code

    Ok(())
}
