use crate::domain::event::SessionOperationalNotification;
use crate::domain::webdriver::Capabilities;
use crate::domain::SessionMetadata;
use crate::harness::Service;
use crate::library::communication::event::{Consumer, NotificationFrame};
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;
use mongodb::Collection;
use tracing::{debug, trace};

pub struct OperationalWatcherService {
    collection: Collection<SessionMetadata>,
}

impl<F> Service<F> for OperationalWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "OperationalWatcherService";

    type Instance = OperationalWatcherService;
    type Config = Collection<SessionMetadata>;

    fn instantiate(_factory: F, collection: &Self::Config) -> Self::Instance {
        Self {
            collection: collection.clone(),
        }
    }
}

#[async_trait]
impl Consumer for OperationalWatcherService {
    type Notification = SessionOperationalNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        debug!(id = ?notification.id, capabilities = ?notification.actual_capabilities, "Session became alive");
        let capabilities: Capabilities = serde_json::from_str(&notification.actual_capabilities)?;

        self.collection
            .update_one(
                mongodb::bson::doc! { "_id": notification.id },
                mongodb::bson::doc! {
                    "$set": {
                        "browserName": capabilities.browser_name,
                        "browserVersion": capabilities.browser_version,
                        "operationalAt": notification.publication_time(),
                    }
                },
                None,
            )
            .await?;

        trace!("Patched metadata object");

        Ok(())
    }
}
