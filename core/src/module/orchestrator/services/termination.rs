use super::ProvisioningState;
use crate::domain::event::SessionTerminatedNotification;
use crate::harness::Service;
use crate::library::communication::event::Consumer;
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;

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

    async fn consume(&self, notification: Self::Notification) -> EmptyResult {
        self.state.release_permit(&notification.id).await;
        Ok(())
    }
}
