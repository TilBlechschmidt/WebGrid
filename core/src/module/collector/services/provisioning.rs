use crate::domain::event::SessionProvisionedNotification;
use crate::domain::SessionMetadata;
use crate::harness::Service;
use crate::library::communication::event::{Consumer, NotificationFrame};
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;
use mongodb::Collection;

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

        Ok(())
    }
}
