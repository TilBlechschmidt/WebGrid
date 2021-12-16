//! Manages a WebDriver instance and translates requests

use anyhow::anyhow;
use async_trait::async_trait;
use domain::event::{
    ModuleTerminationReason, SessionClientMetadata, SessionOperationalNotification,
    SessionTerminatedNotification, SessionTerminationReason,
};
use domain::webdriver::{Capabilities, WebDriver, WebDriverInstance};
use domain::WebgridServiceDescriptor;
use harness::{
    DummyResourceHandleProvider, Heart, HeartStone, Module, RedisCommunicationFactory,
    RedisServiceAdvertisementJob,
};
use jatsl::{schedule, schedule_and_wait, JobScheduler};
use library::communication::event::NotificationPublisher;
use library::communication::{BlackboxError, CommunicationFactory};
use library::storage::s3::S3StorageBackend;
use library::{
    AccumulatedPerformanceMetrics, BoxedError, EmptyResult, PerformanceMonitor,
    PerformanceMonitoringTarget,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::sync::Mutex;
use tracing::{error, info};

mod metadata;
mod options;
mod proxy;
mod recording;

pub use options::Options;

use self::metadata::MetadataPublisherJob;
use self::proxy::ProxyJob;
use self::recording::RecordingJob;

#[derive(Debug, Error)]
enum NodeError {
    #[error("failed to publish SessionOperationalNotification")]
    OperationalNotificationUndeliverable(#[source] BoxedError),
    #[error("attempted to access non-initialized driver instance")]
    DriverNotInitialized,
}

/// Module implementation
pub struct Node {
    options: Options,
    instance: Option<WebDriverInstance>,
    video_byte_count_total: Arc<AtomicUsize>,
    profiling_tx: mpsc::UnboundedSender<Arc<Mutex<AccumulatedPerformanceMetrics>>>,
    profiling_rx: mpsc::UnboundedReceiver<Arc<Mutex<AccumulatedPerformanceMetrics>>>,
}

impl Node {
    /// Creates a new instance from raw parts
    pub fn new(options: Options) -> Self {
        let (profiling_tx, profiling_rx) = mpsc::unbounded_channel();

        Self {
            options,
            instance: None,
            video_byte_count_total: Arc::new(AtomicUsize::new(0)),
            profiling_tx,
            profiling_rx,
        }
    }

    /// Creates a notification publisher with no guarantees regarding delivery
    fn create_oneshot_notification_publisher(&self) -> impl NotificationPublisher {
        let communication_factory = RedisCommunicationFactory::new(
            self.options.redis.url.clone(),
            DummyResourceHandleProvider::new(),
        );

        communication_factory.notification_publisher()
    }

    async fn monitor_process(&self) -> EmptyResult {
        let interval = self.options.profiler_sampling_interval;

        #[cfg(target_os = "linux")]
        match PerformanceMonitor::observe_by_name("Xvfb".into(), "xvfb".into(), interval).await {
            Ok(xvfb_metrics) => {
                self.profiling_tx.send(xvfb_metrics)?;
            }
            Err(error) => {
                error!(?error, "Could not find Xvfb process for monitoring");
            }
        }

        let webgrid_metrics = PerformanceMonitor::observe_self("webgrid".into(), interval).await?;
        let cgroup_metrics = PerformanceMonitor::observe(
            PerformanceMonitoringTarget::CurrentCgroup,
            "cgroup".into(),
            interval,
        );

        self.profiling_tx.send(webgrid_metrics)?;
        self.profiling_tx.send(cgroup_metrics)?;

        Ok(())
    }

    async fn build_heart(&self, capabilities: &Capabilities) -> (Heart, HeartStone) {
        let idle_timeout = if let Some(idle_timeout_secs) = capabilities
            .webgrid_options
            .as_ref()
            .map(|w| w.idle_timeout)
            .flatten()
        {
            Duration::from_secs(idle_timeout_secs)
        } else {
            self.options.idle_timeout
        };

        let (mut heart, stone) = Heart::with_lifetime(idle_timeout);

        heart
            .reduce_next_lifetime(self.options.initial_timeout)
            .await;

        (heart, stone)
    }

    async fn start_driver(&mut self) -> EmptyResult {
        info!("Starting webdriver");
        let webdriver = WebDriver::default()
            .binary(&self.options.webdriver.binary)
            .variant(self.options.webdriver.variant)
            .resolution(self.options.webdriver.resolution)
            .startup_timeout(self.options.webdriver.startup_timeout)
            .capabilities(&self.options.webdriver.capabilities)
            .launch()
            .await?;

        self.instance = Some(webdriver);

        Ok(())
    }

    async fn monitor_driver(&self) -> EmptyResult {
        if let Some(pid) = self.instance.as_ref().map(|w| w.pid()).flatten() {
            let metrics = PerformanceMonitor::observe_by_pid(
                pid as i32,
                "webdriver".into(),
                self.options.profiler_sampling_interval,
            )
            .await?;
            self.profiling_tx.send(metrics)?;
        }

        Ok(())
    }

    async fn monitor_browser(&self) {
        if let Some(pid) = self.instance.as_ref().map(|w| w.pid()).flatten() {
            let profiling_tx = self.profiling_tx.clone();
            let interval = self.options.profiler_sampling_interval;
            let mut monitored_children = Vec::new();

            tokio::spawn(async move {
                loop {
                    let children =
                        PerformanceMonitor::recursively_find_child_processes_of_pid(pid as i32)
                            .await;

                    match children {
                        Ok(children) => {
                            for child in children {
                                let pid = child.pid();
                                if monitored_children.contains(&pid) {
                                    continue;
                                }

                                let postfix = child
                                    .name()
                                    .await
                                    .unwrap_or_else(|_| "unknown".into())
                                    .replace(" ", "");

                                let child_metrics = PerformanceMonitor::observe(
                                    PerformanceMonitoringTarget::Process(child),
                                    format!("browser-{}-{}", pid, postfix),
                                    interval,
                                );

                                monitored_children.push(pid);

                                if let Err(error) = profiling_tx.send(child_metrics) {
                                    error!(?error, "failed to submit browser process metrics");
                                }
                            }
                        }
                        Err(error) => error!(?error, "failed to fetch children of browser"),
                    }

                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            });
        } else {
            error!("failed to monitor browser process as no webdriver pid could be found");
        }
    }

    fn build_proxy_job(
        &self,
        heart_stone: HeartStone,
        metadata_tx: UnboundedSender<SessionClientMetadata>,
    ) -> Result<ProxyJob, NodeError> {
        let instance = match &self.instance {
            Some(instance) => instance,
            None => return Err(NodeError::DriverNotInitialized),
        };

        let authority = instance.socket_addr().to_string();
        let session_id_internal = instance.session_id().to_owned();
        let session_id_external = self.options.id.to_string();
        let identifier = format!("node-{}", session_id_external);

        Ok(ProxyJob::new(
            crate::constants::PORT_NODE,
            identifier,
            authority,
            session_id_internal,
            session_id_external,
            heart_stone,
            metadata_tx,
        ))
    }

    fn build_advertise_job(&self) -> RedisServiceAdvertisementJob<WebgridServiceDescriptor> {
        let endpoint = format!("{}:{}", self.options.host, crate::constants::PORT_NODE);

        RedisServiceAdvertisementJob::new(
            self.options.redis.url.clone(),
            WebgridServiceDescriptor::Node(self.options.id),
            endpoint,
        )
    }

    fn build_metadata_publisher_job(
        &self,
    ) -> (MetadataPublisherJob, UnboundedSender<SessionClientMetadata>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            MetadataPublisherJob::new(self.options.id, rx, self.options.redis.url.clone()),
            tx,
        )
    }

    fn build_recording_job(
        &self,
        capabilities: Capabilities,
    ) -> Option<RecordingJob<S3StorageBackend>> {
        if capabilities
            .webgrid_options
            .map(|w| w.disable_recording)
            .unwrap_or(false)
        {
            None
        } else {
            let interval = if self.options.profile {
                Some(self.options.profiler_sampling_interval)
            } else {
                None
            };

            Some(RecordingJob::new(
                self.options.id,
                self.options.recording.generate_arguments(),
                self.options.storage.backend.clone()?,
                self.video_byte_count_total.clone(),
                self.profiling_tx.clone(),
                interval,
            ))
        }
    }

    async fn send_alive_notification(&self) -> Result<(), NodeError> {
        info!("Reached operational state");
        if let Some(driver) = &self.instance {
            let publisher = self.create_oneshot_notification_publisher();
            let notification = SessionOperationalNotification {
                id: self.options.id,
                actual_capabilities: driver.capabilities().to_owned(),
            };

            if let Err(e) = publisher.publish(&notification).await {
                // TODO Maybe retry instead of bailing out. It is quite expensive to get a session up and running,
                //      so discarding it due to a potentially intermittent network failure seems like a waste.
                Err(NodeError::OperationalNotificationUndeliverable(e))
            } else {
                Ok(())
            }
        } else {
            unreachable!("Attempted to retrieve capabilities for a non-existing driver instance")
        }
    }

    async fn shutdown_driver(&mut self) {
        info!("Shutting down driver");
        if let Some(driver) = self.instance.take() {
            driver.kill().await.ok();
        }
    }

    async fn collect_profiling_data(&mut self) -> HashMap<String, AccumulatedPerformanceMetrics> {
        let mut profiling_data = HashMap::new();

        while let Ok(entry) = self.profiling_rx.try_recv() {
            let data = entry.lock().await.clone();
            profiling_data.insert(data.label.clone(), data);
        }

        profiling_data
    }

    async fn send_termination_notification(&mut self, reason: SessionTerminationReason) {
        info!("Publishing termination notification");
        let publisher = self.create_oneshot_notification_publisher();

        // If we terminated before being "operational" then send out a SessionStartupFailedNotification, else publish a SessionTerminatedNotification
        let result = match reason {
            SessionTerminationReason::StartupFailed { error: cause } => {
                let notification =
                    SessionTerminatedNotification::new_for_startup_failure(self.options.id, cause);

                publisher.publish(&notification).await
            }
            SessionTerminationReason::ModuleTimeout => {
                // Since this is run in the shutdown routine, the ModuleTimeout can only be caused by a failed startup routine
                let notification = SessionTerminatedNotification::new_for_startup_failure(
                    self.options.id,
                    BlackboxError::from_boxed(anyhow!("session startup routine timed out").into()),
                );

                publisher.publish(&notification).await
            }
            _ => {
                let recording_bytes = self.video_byte_count_total.load(Ordering::Relaxed);
                let profiling_data = self.collect_profiling_data().await;

                let notification = SessionTerminatedNotification {
                    id: self.options.id,
                    reason,
                    recording_bytes,
                    profiling_data,
                };

                publisher.publish(&notification).await
            }
        };

        if let Err(error) = result {
            error!(?error, "Failed to broadcast termination notification");
        }
    }
}

#[async_trait]
impl Module for Node {
    async fn pre_startup(&mut self) -> EmptyResult {
        self.start_driver().await?;

        if self.options.profile {
            self.monitor_process().await?;
            self.monitor_driver().await?;
            self.monitor_browser().await;
        }

        Ok(())
    }

    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let capabilities: Capabilities =
            serde_json::from_str(&self.options.webdriver.capabilities)?;
        let (heart, stone) = self.build_heart(&capabilities).await;

        let advertise_job = self.build_advertise_job();
        let (metadata_publisher_job, metadata_tx) = self.build_metadata_publisher_job();
        let proxy_job = self.build_proxy_job(stone, metadata_tx)?;

        if let Some(recording_job) = self.build_recording_job(capabilities) {
            schedule!(scheduler, { recording_job });
        }

        schedule_and_wait!(scheduler, self.options.bind_timeout, {
            proxy_job,
            advertise_job,
            metadata_publisher_job
        });

        // TODO The ready-states are not yet indicative of the actual ready state as e.g. the HTTP server future is only polled after the ready signal is sent.
        self.send_alive_notification().await?;

        Ok(Some(heart))
    }

    async fn post_shutdown(&mut self, termination_reason: ModuleTerminationReason) {
        // These are only best-effort cleanup attempts. They may very well fail for one reason or another.
        self.shutdown_driver().await;
        self.send_termination_notification(termination_reason.into())
            .await;
    }
}
