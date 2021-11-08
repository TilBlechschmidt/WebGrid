use async_trait::async_trait;
use domain::event::SessionScheduledNotification;
use domain::SessionMetadata;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use mongodb::Collection;
use tracing::{debug, trace};

pub struct SchedulingWatcherService {
    collection: Collection<SessionMetadata>,
}

impl<F> Service<F> for SchedulingWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "SchedulingWatcherService";

    type Instance = SchedulingWatcherService;
    type Config = Collection<SessionMetadata>;

    fn instantiate(_factory: F, collection: &Self::Config) -> Self::Instance {
        Self {
            collection: collection.clone(),
        }
    }
}

#[async_trait]
impl Consumer for SchedulingWatcherService {
    type Notification = SessionScheduledNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        debug!(id = ?notification.id, provisioner = ?notification.provisioner, scheduled_at = ?notification.publication_time(), "Session scheduled with provisioner");

        self.collection
            .update_one(
                mongodb::bson::doc! { "_id": notification.id },
                mongodb::bson::doc! {
                    "$set": {
                        "scheduledAt": notification.publication_time(),
                        "provisioner": notification.into_inner().provisioner
                    }
                },
                None,
            )
            .await?;

        trace!("Patched metadata object");

        Ok(())
    }
}
