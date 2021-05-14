//! Unified grid entrypoint provider

use super::SharedOptions;
use crate::libraries::{
    helpers::constants,
    tracing::{self, constants::service},
};
use crate::libraries::{lifecycle::Heart, net::discovery::ServiceDiscovery};
use anyhow::Result;
use jatsl::{schedule, JobScheduler, StatusServer};
use log::info;
use structopt::StructOpt;

mod context;
mod jobs;

use context::Context;
use jobs::ProxyJob;

#[derive(Debug, StructOpt)]
/// Unified grid entrypoint provider
///
/// Provides a unified endpoint which routes traffic to available managers or requested sessions.
pub struct Options {
    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_PROXY)]
    port: u16,

    /// Size of the service endpoint cache
    ///
    /// It makes sense to set this slightly higher than the maximum number of concurrently active sessions.
    #[structopt(short, long, default_value = "42")]
    cache_size: usize,
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    tracing::init(&shared_options.trace_endpoint, service::PROXY, None)?;

    let (mut heart, _) = Heart::new();

    let context = Context::new(shared_options.redis);
    let scheduler = JobScheduler::default();

    let (discovery, discovery_job) = ServiceDiscovery::new(100, options.cache_size);
    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let metrics_job = context.metrics.clone();
    let proxy_job = ProxyJob::new(options.port, discovery);

    schedule!(scheduler, context, {
        status_job,
        metrics_job,
        proxy_job,
        discovery_job,
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
