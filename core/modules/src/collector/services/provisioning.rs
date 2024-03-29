use async_trait::async_trait;
use domain::event::SessionProvisionedNotification;
use domain::SessionMetadata;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use mongodb::Collection;
use tracing::{debug, trace};

pub struct ProvisioningWatcherService {
    collection: Collection<SessionMetadata>,
}

impl<F> Service<F> for ProvisioningWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "ProvisioningWatcherService";

    type Instance = ProvisioningWatcherService;
    type Config = Collection<SessionMetadata>;

    fn instantiate(_factory: F, collection: &Self::Config) -> Self::Instance {
        Self {
            collection: collection.clone(),
        }
    }
}

#[async_trait]
impl Consumer for ProvisioningWatcherService {
    type Notification = SessionProvisionedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        debug!(id = ?notification.id, provisioned_at = ?notification.publication_time(), "Session provisioned");

        self.collection
            .update_one(
                mongodb::bson::doc! { "_id": notification.id },
                mongodb::bson::doc! {
                    "$set": {
                        "provisionedAt": notification.publication_time()
                    }
                },
                None,
            )
            .await?;

        trace!("Patched metadata object");

        Ok(())
    }
}
