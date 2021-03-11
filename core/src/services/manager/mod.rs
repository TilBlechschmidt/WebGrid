//! Endpoint for handling session creation

use super::SharedOptions;
use crate::libraries::helpers::constants;
use crate::libraries::lifecycle::Heart;
use crate::libraries::scheduling::{JobScheduler, StatusServer};
use crate::schedule;
use log::info;
use structopt::StructOpt;

mod context;
mod jobs;
mod structures;
mod tasks;

use context::Context;
use jobs::SessionHandlerJob;
pub use structures::*;

#[derive(Debug, StructOpt)]
/// Endpoint for handling session creation
///
/// Handles scheduling and provisioning lifecycle of sessions.
pub struct Options {
    /// Unique instance identifier
    #[structopt(env)]
    id: String,

    /// Host under which the manager is reachable by other services
    #[structopt(env = "MANAGER_HOST")]
    host: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_MANAGER)]
    port: u16,
}

pub async fn run(shared_options: SharedOptions, options: Options) {
    let (mut heart, _) = Heart::new();

    let host = format!("{}:{}", options.host, options.port);
    let context = Context::new(shared_options.redis, host);
    let scheduler = JobScheduler::default();

    context.spawn_heart_beat(&options.id, &scheduler).await;

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let session_handler_job = SessionHandlerJob::new(options.port);

    schedule!(scheduler, context, {
        status_job,
        session_handler_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;
}
