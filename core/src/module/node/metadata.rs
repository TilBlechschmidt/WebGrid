use crate::domain::event::{
    SessionClientMetadata, SessionIdentifier, SessionMetadataModifiedNotification,
};
use crate::harness::RedisCommunicationFactory;
use crate::library::communication::event::NotificationPublisher;
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use anyhow::anyhow;
use async_trait::async_trait;
use jatsl::{Job, JobManager};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub struct MetadataPublisherJob {
    id: SessionIdentifier,
    receiver: Mutex<mpsc::UnboundedReceiver<SessionClientMetadata>>,
    redis_url: String,
}

impl MetadataPublisherJob {
    pub fn new(
        id: SessionIdentifier,
        receiver: mpsc::UnboundedReceiver<SessionClientMetadata>,
        redis_url: String,
    ) -> Self {
        Self {
            id,
            receiver: Mutex::new(receiver),
            redis_url,
        }
    }

    async fn publish_notifications<P: NotificationPublisher>(&self, publisher: P) {
        let mut receiver = self.receiver.lock().await;

        while let Some(metadata) = receiver.recv().await {
            let notification = SessionMetadataModifiedNotification {
                id: self.id,
                metadata,
            };

            if let Err(e) = publisher.publish(&notification).await {
                log::error!(
                    "Failed to publish SessionMetadataModifiedNotification: {}",
                    e
                );
            }
        }
    }
}

#[async_trait]
impl Job for MetadataPublisherJob {
    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: JobManager) -> EmptyResult {
        let manager = Arc::new(manager);
        let factory = RedisCommunicationFactory::new(self.redis_url.clone(), manager.clone());
        let publisher = factory.notification_publisher();

        manager.ready().await;

        self.publish_notifications(publisher).await;

        if manager.termination_signal_triggered() {
            Ok(())
        } else {
            Err(
                anyhow!("Unexpected termination of supposedly infinite MetadataPublisherJob loop")
                    .into(),
            )
        }
    }
}
