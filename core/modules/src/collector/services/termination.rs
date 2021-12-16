use async_trait::async_trait;
use domain::event::SessionTerminatedNotification;
use domain::SessionMetadata;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use mongodb::Collection;
use tracing::debug;

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
        debug!(id = ?notification.id, terminated_at = ?notification.publication_time(), reason = ?notification.reason, "Session terminated");

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
        metadata.profiling_data = notification.profiling_data;

        self.collection.insert_one(metadata, None).await?;
        self.staging_collection.delete_one(query, None).await?;

        Ok(())
    }
}
