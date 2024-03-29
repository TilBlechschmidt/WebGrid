use super::ProvisioningState;
use async_trait::async_trait;
use domain::event::SessionTerminatedNotification;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use tracing::debug;

/// Watches for terminated sessions and releases the semaphore permits held by them
pub struct SessionTerminationWatcherService {
    state: ProvisioningState,
}

impl<F> Service<F> for SessionTerminationWatcherService
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "SessionTerminationWatcherService";
    type Instance = SessionTerminationWatcherService;
    type Config = ProvisioningState;

    fn instantiate(_factory: F, state: &Self::Config) -> Self::Instance {
        Self {
            state: state.clone(),
        }
    }
}

#[async_trait]
impl Consumer for SessionTerminationWatcherService {
    type Notification = SessionTerminatedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        debug!(id = ?notification.id, "Session terminated, releasing permit");
        self.state.release_permit(&notification.id).await;
        Ok(())
    }
}
