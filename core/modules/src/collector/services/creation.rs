use async_trait::async_trait;
use domain::event::SessionCreatedNotification;
use domain::SessionMetadata;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use mongodb::Collection;
use tracing::{debug, trace};

pub struct CreationWatcherService {
    collection: Collection<SessionMetadata>,
}

impl<F> Service<F> for CreationWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "CreationWatcherService";

    type Instance = CreationWatcherService;
    type Config = Collection<SessionMetadata>;

    fn instantiate(_factory: F, collection: &Self::Config) -> Self::Instance {
        Self {
            collection: collection.clone(),
        }
    }
}

#[async_trait]
impl Consumer for CreationWatcherService {
    type Notification = SessionCreatedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        debug!(id = ?notification.id, created_at = ?notification.publication_time(), capabilities = ?notification.capabilities, "New session created");
        let mut metadata = SessionMetadata::new(notification.id);

        metadata.created_at = Some(notification.publication_time().to_owned());

        self.collection.insert_one(metadata, None).await?;
        trace!("Inserted new metadata object");

        Ok(())
    }
}
