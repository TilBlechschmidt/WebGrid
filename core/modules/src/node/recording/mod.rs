use async_trait::async_trait;
use domain::{event::SessionIdentifier, storage_path};
use futures::join;
use heim::process;
use heim::process::os::unix::{ProcessExt, Signal};
use hyper::Server;
use jatsl::Job;
use library::storage::StorageBackend;
use library::{http::Responder, make_responder_chain_service_fn, responder_chain};
use library::{AccumulatedPerformanceMetrics, EmptyResult, PerformanceMonitor};
use std::io::Read;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use storage::StorageResponder;
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{error, info};

mod storage;

#[derive(Debug, Error)]
enum RecordingError {
    #[error("no pid found for ffmpeg process")]
    NoPIDFound,
    #[error("attempted to restart a previously started recording")]
    NotRestartable,
}

pub struct RecordingJob<S: StorageBackend> {
    arguments: String,
    storage: S,
    session_id: SessionIdentifier,
    started: AtomicBool,
    byte_count_total: Arc<AtomicUsize>,
    profiling_tx: mpsc::UnboundedSender<Arc<Mutex<AccumulatedPerformanceMetrics>>>,
    profiling_interval: Option<Duration>,
}

impl<S> RecordingJob<S>
where
    S: StorageBackend + Send + Sync + 'static,
{
    pub fn new(
        session_id: SessionIdentifier,
        arguments: String,
        storage: S,
        byte_count_total: Arc<AtomicUsize>,
        profiling_tx: mpsc::UnboundedSender<Arc<Mutex<AccumulatedPerformanceMetrics>>>,
        profiling_interval: Option<Duration>,
    ) -> Self {
        Self {
            arguments,
            storage,
            session_id,
            started: AtomicBool::new(false),
            byte_count_total,
            profiling_tx,
            profiling_interval,
        }
    }
}

#[async_trait]
impl<S> Job for RecordingJob<S>
where
    S: StorageBackend + Send + Sync + 'static,
{
    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: jatsl::JobManager) -> EmptyResult {
        let storage_responder = StorageResponder::new(
            self.session_id,
            self.storage.clone(),
            self.byte_count_total.clone(),
        );
        let make_svc = make_responder_chain_service_fn!(storage_responder);

        let addr = SocketAddr::from(([127, 0, 0, 1], crate::constants::PORT_STORAGE));
        let server = Server::try_bind(&addr)?
            .http1_half_close(true)
            .serve(make_svc);

        // Make sure the recording is not restarted as that would override all previous footage
        if self.started.load(Ordering::Acquire) {
            return Err(RecordingError::NotRestartable.into());
        }

        // Prepare the log output file
        let log_file_writeable = NamedTempFile::new()?;
        let mut log_file_readable = log_file_writeable.reopen()?;

        // Spawn the recording subprocess
        let args: Vec<&str> = self.arguments.split_whitespace().collect();
        info!("Launching ffmpeg {}", args.join(" "));
        let mut ffmpeg = Command::new("ffmpeg")
            .args(&args)
            .stderr(log_file_writeable.into_file())
            .spawn()?;

        // Signal our readiness
        // TODO Bind readiness to the first incoming HTTP request (that way we know ffmpeg actually works)
        manager.ready().await;
        self.started.store(true, Ordering::Release);

        // Prepare the three step termination process
        let pid = ffmpeg.id().ok_or(RecordingError::NoPIDFound)? as i32;
        let process = process::get(pid).await?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Profile ffmpeg (if enabled)
        if let Some(interval) = self.profiling_interval {
            let metrics =
                PerformanceMonitor::observe_by_pid(pid, "ffmpeg".into(), interval).await?;
            self.profiling_tx.send(metrics)?;
        }

        // 1. Wait for an external termination signal and send SIGTERM to ffmpeg
        let termination_request = async move {
            manager.termination_signal().await;
            if let Err(e) = process.signal(Signal::Term).await {
                error!("Failed to send SIGTERM to ffmepg: {}", e);
            }
        };

        // 2. Wait for ffmpeg to finish up the recording in response to the SIGTERM
        let ffmpeg_termination = async move {
            if let Err(e) = ffmpeg.wait().await {
                error!("Failed to await ffmpeg termination: {}", e);
            }
            if shutdown_tx.send(()).is_err() {
                error!("Failed to trigger shutdown of video forwarding HTTP server");
            }
        };

        // 3. Once ffmpeg has terminated, shutdown the HTTP server by "abusing" the graceful shutdown for our purposes :D
        let server_shutdown = async move {
            let shutdown_result = server
                .with_graceful_shutdown(async move {
                    if shutdown_rx.await.is_err() {
                        error!("Failed to receive video forwarding termination signal");
                    }
                })
                .await;

            if let Err(e) = shutdown_result {
                error!("Video forwarding server terminated with error: {}", e);
            }
        };

        // Poll all three futures to completion
        join! {
            termination_request,
            ffmpeg_termination,
            server_shutdown,
        };

        // Upload the ffmpeg logs
        let mut ffmpeg_logs = Vec::new();
        let log_object_path: String = storage_path(self.session_id, "ffmpeg.log")
            .to_string_lossy()
            .into_owned();
        log_file_readable.read_to_end(&mut ffmpeg_logs)?;
        self.storage
            .put_object(&log_object_path, &ffmpeg_logs)
            .await?;

        Ok(())
    }
}
