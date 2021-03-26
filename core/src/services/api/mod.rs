//! GraphQL API service

use super::SharedOptions;
use crate::libraries::helpers::constants;
use crate::libraries::lifecycle::Heart;
use crate::libraries::scheduling::{JobScheduler, StatusServer};
use crate::schedule;
use anyhow::Result;
use jobs::ServerJob;
use log::info;
use structopt::StructOpt;

mod context;
mod jobs;

use context::Context;
use uuid::Uuid;

#[derive(Debug, StructOpt)]
/// GraphQL API service
///
/// Provides an external API that allows external access to the grid status.
pub struct Options {
    /// Unique instance identifier
    #[structopt(env)]
    id: Option<String>,

    /// Host under which the api server is reachable by the proxy
    #[structopt(long, env)]
    host: String,

    /// Port on which the HTTP server will listen
    #[structopt(short, long, default_value = constants::PORT_API)]
    port: u16,
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let (mut heart, _) = Heart::new();
    let api_id = options
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if options.id.is_none() {
        info!("Randomly generated service identifier: {}", api_id);
    }

    let context = Context::new(&options, shared_options.redis, &api_id).await;
    let scheduler = JobScheduler::default();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let heart_beat_job = context.heart_beat.clone();
    let server_job = ServerJob::new(options.port);

    schedule!(scheduler, context, {
        status_job,
        heart_beat_job,
        server_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}
