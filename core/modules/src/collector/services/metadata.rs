use async_trait::async_trait;
use domain::event::SessionMetadataModifiedNotification;
use domain::SessionMetadata;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use mongodb::bson::Document;
use mongodb::Collection;
use tracing::{debug, trace};

pub struct MetadataWatcherService {
    collection: Collection<SessionMetadata>,
}

impl<F> Service<F> for MetadataWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "MetadataWatcherService";

    type Instance = MetadataWatcherService;
    type Config = Collection<SessionMetadata>;

    fn instantiate(_factory: F, collection: &Self::Config) -> Self::Instance {
        Self {
            collection: collection.clone(),
        }
    }
}

#[async_trait]
impl Consumer for MetadataWatcherService {
    type Notification = SessionMetadataModifiedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        let mut update = Document::new();

        debug!(id = ?notification.id, metadata = ?notification.metadata, "Received session metadata patch");

        for (key, value) in notification.metadata.iter() {
            update.insert(format!("clientMetadata.{}", key), value);
        }

        self.collection
            .update_one(
                mongodb::bson::doc! { "_id": notification.id },
                mongodb::bson::doc! {
                    "$set": update
                },
                None,
            )
            .await?;

        trace!("Patched metadata object");

        Ok(())
    }
}
