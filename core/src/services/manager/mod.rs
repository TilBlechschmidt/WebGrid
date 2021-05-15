//! Endpoint for handling session creation

use std::time::Duration;

use super::SharedOptions;
use crate::libraries::{
    helpers::{constants, parse_seconds},
    tracing::{self, constants::service},
};
use crate::libraries::{
    lifecycle::Heart,
    net::{advertise::ServiceAdvertisorJob, discovery::ServiceDescriptor},
};
use anyhow::Result;
use jatsl::{schedule, JobScheduler, StatusServer};
use log::info;
use structopt::StructOpt;

mod context;
mod jobs;
mod structures;
mod tasks;

use context::Context;
use jobs::SessionHandlerJob;
pub use structures::*;

#[derive(Debug, StructOpt, Clone)]
/// Endpoint for handling session creation
///
/// Handles scheduling and provisioning lifecycle of sessions.
pub struct Options {
    /// Unique instance identifier
    #[structopt(env)]
    id: String,

    /// Host under which the manager server is reachable by the proxy
    #[structopt(long, env)]
    host: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_MANAGER)]
    port: u16,

    /// Maximum duration to wait in queue; in seconds
    #[structopt(long, env, default_value = "600", parse(try_from_str = parse_seconds))]
    timeout_queue: Duration,

    /// Maximum duration to wait for a session to become provisioned; in seconds
    #[structopt(long, env, default_value = "300", parse(try_from_str = parse_seconds))]
    timeout_provisioning: Duration,
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    tracing::init(
        &shared_options.trace_endpoint,
        service::MANAGER,
        Some(&options.id),
    )?;

    let (mut heart, _) = Heart::new();

    let endpoint = format!("{}:{}", options.host, options.port);
    let context = Context::new(shared_options.redis, options.clone()).await;
    let scheduler = JobScheduler::default();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let heart_beat_job = context.heart_beat.clone();
    let metrics_job = context.metrics.clone();
    let session_handler_job = SessionHandlerJob::new(options.port);
    let advertise_job = ServiceAdvertisorJob::new(ServiceDescriptor::Manager, endpoint);

    schedule!(scheduler, context, {
        status_job,
        heart_beat_job,
        metrics_job,
        session_handler_job,
        advertise_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
