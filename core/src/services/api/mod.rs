//! GraphQL API service

use super::SharedOptions;
use crate::libraries::lifecycle::Heart;
use crate::libraries::{
    helpers::constants,
    net::{advertise::ServiceAdvertisorJob, discovery::ServiceDescriptor},
};
use anyhow::Result;
use jatsl::{schedule, JobScheduler, StatusServer};
use jobs::ServerJob;
use log::info;
use std::path::PathBuf;
use structopt::StructOpt;

mod context;
mod jobs;
mod schema;

use context::Context;

#[derive(Debug, StructOpt)]
/// GraphQL API service
///
/// Provides an external API that allows external access to the grid status.
pub struct Options {
    /// Host under which the api server is reachable by the proxy
    #[structopt(long, env)]
    host: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_API)]
    port: u16,

    /// Directory from which files will be served
    #[structopt(long, env, parse(from_os_str), default_value = "/www")]
    web_root: PathBuf,
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let (mut heart, _) = Heart::new();
    let endpoint = format!("{}:{}", options.host, options.port);

    let context = Context::new(&options, shared_options.redis).await;
    let scheduler = JobScheduler::default();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let server_job = ServerJob::new(options.port);
    let advertise_job = ServiceAdvertisorJob::new(ServiceDescriptor::Api, endpoint);

    schedule!(scheduler, context, {
        status_job,
        server_job,
        advertise_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
