use crate::domain::event::SessionCreatedNotification;
use crate::harness::RedisCommunicationFactory;
use crate::library::communication::event::NotificationPublisher;
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use anyhow::anyhow;
use async_trait::async_trait;
use jatsl::{Job, JobManager};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub struct CreatedNotificationPublisherJob {
    receiver: Mutex<mpsc::UnboundedReceiver<SessionCreatedNotification>>,
    redis_url: String,
}

impl CreatedNotificationPublisherJob {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<SessionCreatedNotification>,
        redis_url: String,
    ) -> Self {
        Self {
            receiver: Mutex::new(receiver),
            redis_url,
        }
    }

    async fn publish_notifications<P: NotificationPublisher>(&self, publisher: P) {
        let mut receiver = self.receiver.lock().await;

        while let Some(notification) = receiver.recv().await {
            if let Err(e) = publisher.publish(&notification).await {
                log::error!("Failed to publish SessionCreatedNotification: {}", e);
            }
        }
    }
}

#[async_trait]
impl Job for CreatedNotificationPublisherJob {
    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: JobManager) -> EmptyResult {
        let handle_provider = Arc::new(manager.clone());
        let factory = RedisCommunicationFactory::new(self.redis_url.clone(), handle_provider);
        let publisher = factory.notification_publisher();

        self.publish_notifications(publisher).await;

        Err(anyhow!(
            "Unexpected termination of supposedly infinite CreatedNotificationPublisher loop"
        )
        .into())
    }
}
