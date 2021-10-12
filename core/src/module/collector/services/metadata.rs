use crate::domain::event::SessionMetadataModifiedNotification;
use crate::domain::SessionMetadata;
use crate::harness::Service;
use crate::library::communication::event::{Consumer, NotificationFrame};
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;
use mongodb::bson::Document;
use mongodb::Collection;

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

        Ok(())
    }
}
