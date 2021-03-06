//! Session provider and driver manager

use super::SharedOptions;
use crate::{
    libraries::{
        helpers::{constants, parse_seconds},
        net::{advertise::ServiceAdvertisorJob, discovery::ServiceDescriptor},
        recording::VideoQualityPreset,
        tracing::{self, constants::service},
    },
    services::node::tasks::initialize_tracing,
};
use anyhow::Result;
use jatsl::{schedule, JobScheduler, StatusServer};
use log::{info, warn};
use opentelemetry::trace::TraceContextExt;
use std::{path::PathBuf, time::Duration};
use structopt::StructOpt;
use uuid::Uuid;

mod context;
mod jobs;
mod structs;
mod tasks;

use context::Context;
use jobs::{ProxyJob, RecorderJob};
use tasks::{initialize_service, initialize_session, start_driver, stop_driver, terminate};

#[derive(Debug, StructOpt, Clone)]
/// Session provider
///
/// Manages the lifecycle of one session in terms of interaction with the driver and screen recording.
/// Bound to the lifecycle of one session and usually not started explicitly but through a provisioner like Docker or Kubernetes.
pub struct Options {
    /// Unique instance identifier
    #[structopt(env)]
    id: Uuid,

    /// Host under which the node server is reachable by the proxy
    #[structopt(long, env)]
    host: String,

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

    /// Directory in which to store video recordings
    ///
    /// Omitting this option will disable video recording!
    #[structopt(long, env, parse(from_os_str))]
    storage_directory: Option<PathBuf>,

    /// Framerate of the video input defined by --recording-input
    ///
    /// Note that this *does not* set the framerate but is rather the framerate the input you pass in has.
    #[structopt(long, env, default_value = "5")]
    recording_framerate: usize,

    /// ffmpeg input parameter specification
    #[structopt(
        long,
        env,
        default_value = "-rtbufsize 1500M -probesize 100M -framerate 5 -video_size 1920x1080 -f x11grab -i :42"
    )]
    recording_input: String,

    /// Constant Rate Factor
    ///
    /// The range of the CRF scale is 0–51, where 0 is lossless, 23 is the default, and 51 is worst quality possible.
    /// A lower value generally leads to higher quality, and a subjectively sane range is 17–28.
    /// Consider 17 or 18 to be visually lossless or nearly so; it should look the same or nearly the same as the input but it isn't technically lossless.
    /// The range is exponential, so increasing the CRF value +6 results in roughly half the bitrate / file size, while -6 leads to roughly twice the bitrate.
    /// Choose the highest CRF value that still provides an acceptable quality. If the output looks good, then try a higher value. If it looks bad, choose a lower value.
    ///
    /// For more details, consult the ffmpeg H.264 documentation (section "Constant Rate Factor"):
    ///
    /// https://trac.ffmpeg.org/wiki/Encode/H.264
    #[structopt(long, env, default_value = "46")]
    crf: u8,

    /// Upper bitrate bound in bytes
    ///
    /// The average bitrate is determined by the constant rate factor and content
    /// however if the bitrate were to exceed this specified maximum bitrate limit, the codec will increase the CRF temporarily.
    ///
    /// For more details, consult the ffmpeg H.264 documentation (section "Constrained encoding"):
    ///
    /// https://trac.ffmpeg.org/wiki/Encode/H.264
    #[structopt(long, env, default_value = "450000")]
    max_bitrate: usize,

    /// Maximum idle duration after which the node self-terminates; in seconds
    #[structopt(long, env, default_value = "300", parse(try_from_str = parse_seconds))]
    timeout_idle: Duration,

    /// Maximum duration the WebDriver may take to become responsive; in seconds
    #[structopt(long, env, default_value = "30", parse(try_from_str = parse_seconds))]
    timeout_driver_startup: Duration,
}

impl Options {
    fn recording_quality(&self) -> VideoQualityPreset {
        VideoQualityPreset::new(self.crf, self.max_bitrate)
    }
}

async fn launch_session(
    shared_options: SharedOptions,
    options: Options,
    context: &Context,
) -> Result<()> {
    let scheduler = JobScheduler::default();
    let endpoint = format!("{}:{}", options.host, options.port);

    let telemetry_context =
        JobScheduler::spawn_task(&initialize_tracing, context.clone()).await???;

    let startup_context = context
        .clone()
        .with_telemetry_context(telemetry_context.clone());

    // TODO Handle error and go straight to cleanup jobs + make a serial-task-execution macro
    let (mut heart, heart_stone) =
        JobScheduler::spawn_task(&initialize_service, startup_context.clone()).await???;

    JobScheduler::spawn_task(&start_driver, startup_context.clone()).await???;

    let internal_session_id =
        JobScheduler::spawn_task(&initialize_session, startup_context.clone()).await???;

    // End the startup trace
    telemetry_context.span().end();

    let status_job = StatusServer::new(&scheduler, shared_options.status_server);
    let heart_beat_job = context.heart_beat.clone();
    let proxy_job = ProxyJob::new(options.port, internal_session_id, heart_stone);
    let recorder_job = RecorderJob::new();
    let advertise_job = ServiceAdvertisorJob::new(ServiceDescriptor::Node(options.id), endpoint);

    schedule!(scheduler, context, {
        status_job,
        heart_beat_job,
        proxy_job,
        recorder_job,
        advertise_job
    });

    let death_reason = heart.death().await;
    info!("Heart died: {}", death_reason);

    scheduler.terminate_jobs().await;

    Ok(())
}

pub async fn run(shared_options: SharedOptions, options: Options) -> Result<()> {
    let context = Context::new(shared_options.redis.clone(), options.clone()).await;

    tracing::init(
        &shared_options.trace_endpoint,
        service::NODE,
        Some(&options.id.to_string()),
    )?;

    if let Err(e) = launch_session(shared_options, options, &context).await {
        warn!("Encountered error while launching session: {:?}", e);
    }

    JobScheduler::spawn_task(&terminate, context.clone()).await???;
    JobScheduler::spawn_task(&stop_driver, context).await???;

    Ok(())
}
