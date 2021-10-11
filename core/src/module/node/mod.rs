//! Manages a WebDriver instance and translates requests

use crate::domain::event::{
    SessionClientMetadata, SessionOperationalNotification, SessionTerminatedNotification,
    SessionTerminationReason,
};
use crate::domain::webdriver::{Capabilities, WebDriver, WebDriverInstance};
use crate::domain::WebgridServiceDescriptor;
use crate::harness::{
    DummyResourceHandleProvider, Heart, HeartStone, Module, ModuleTerminationReason,
    RedisCommunicationFactory, RedisServiceAdvertisementJob,
};
use crate::library::communication::event::NotificationPublisher;
use crate::library::communication::{BlackboxError, CommunicationFactory};
use crate::library::storage::s3::S3StorageBackend;
use crate::library::{BoxedError, EmptyResult};
use anyhow::anyhow;
use async_trait::async_trait;
use jatsl::{schedule, schedule_and_wait, JobScheduler};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender};

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
}

impl Node {
    /// Creates a new instance from raw parts
    pub fn new(options: Options) -> Self {
        Self {
            options,
            instance: None,
            video_byte_count_total: Arc::new(AtomicUsize::new(0)),
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

    async fn start_driver(&mut self) -> EmptyResult {
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
            Some(RecordingJob::new(
                self.options.id,
                self.options.recording.generate_arguments(),
                self.options.storage.backend.clone()?,
                self.video_byte_count_total.clone(),
            ))
        }
    }

    async fn send_alive_notification(&self) -> Result<(), NodeError> {
        if let Some(driver) = &self.instance {
            let publisher = self.create_oneshot_notification_publisher();
            let notification = SessionOperationalNotification {
                id: self.options.id,
                actual_capabilities: driver.capabilities().to_owned(),
            };

            if let Err(e) = publisher.publish(&notification).await {
                // TODO Maybe retry instead of bailing out. It is quite expensive to get a session up and running,
                //      so discarding it due to a potentially intermittent network failure seems like a waste.
                log::error!("Failed to send SessionOperationalNotification: {}", e);
                Err(NodeError::OperationalNotificationUndeliverable(e))
            } else {
                Ok(())
            }
        } else {
            unreachable!("Attempted to retrieve capabilities for a non-existing driver instance")
        }
    }

    async fn shutdown_driver(&mut self) {
        if let Some(driver) = self.instance.take() {
            if let Err(e) = driver.kill().await {
                log::error!("Failed to kill webdriver instance: {}", e);
            }
        } else {
            log::error!("Attempted to kill an uninitialized webdriver instance!");
        }
    }

    async fn send_termination_notification(&self, reason: SessionTerminationReason) {
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
                let notification = SessionTerminatedNotification {
                    id: self.options.id,
                    reason,
                    recording_bytes,
                };

                publisher.publish(&notification).await
            }
        };

        if let Err(e) = result {
            log::error!("Failed to send SessionTerminatedNotification: {}", e);
        }
    }
}

#[async_trait]
impl Module for Node {
    async fn pre_startup(&mut self) -> EmptyResult {
        self.start_driver().await?;

        // TODO Run post-startup bash script
        //      NOTE: Evaluate if this is really required because it might pose a potential security hazard.

        Ok(())
    }

    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let capabilities: Capabilities =
            serde_json::from_str(&self.options.webdriver.capabilities)?;
        let (mut heart, stone) = Heart::with_lifetime(self.options.idle_timeout);

        heart
            .reduce_next_lifetime(self.options.initial_timeout)
            .await;

        // TODO Spawn process monitoring for webdriver (todo find a generic solution because it won't be the last one)

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
        log::info!("Module terminated with reason: {:?}", termination_reason);

        // These are only best-effort cleanup attempts. They may very well fail for one reason or another.
        self.shutdown_driver().await;
        self.send_termination_notification(termination_reason.into())
            .await;
    }
}
