use crate::domain::event::SessionTerminatedNotification;
use crate::domain::SessionMetadata;
use crate::harness::Service;
use crate::library::communication::event::{Consumer, NotificationFrame};
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;
use mongodb::Collection;

pub struct TerminationWatcherService {
    collection: Collection<SessionMetadata>,
    staging_collection: Collection<SessionMetadata>,
}

impl<F> Service<F> for TerminationWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "TerminationWatcherService";

    type Instance = TerminationWatcherService;
    type Config = (Collection<SessionMetadata>, Collection<SessionMetadata>);

    fn instantiate(_factory: F, collection: &Self::Config) -> Self::Instance {
        Self {
            collection: collection.0.clone(),
            staging_collection: collection.1.clone(),
        }
    }
}

#[async_trait]
impl Consumer for TerminationWatcherService {
    type Notification = SessionTerminatedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        let query = mongodb::bson::doc! { "_id": notification.id };
        let mut metadata = self
            .staging_collection
            .find_one(query.clone(), None)
            .await?
            .unwrap_or_else(|| SessionMetadata::new(notification.id));

        metadata.terminated_at = Some(notification.publication_time().to_owned());

        let notification = notification.into_inner();
        metadata.termination = Some(notification.reason);
        metadata.recording_bytes = Some(notification.recording_bytes as i64);

        self.collection.insert_one(metadata, None).await?;
        self.staging_collection.delete_one(query, None).await?;

        Ok(())
    }
}
