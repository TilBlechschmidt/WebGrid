use std::marker::PhantomData;

use super::super::{SessionCreationCommunicationHandle, StatusResponse};
use async_trait::async_trait;
use domain::event::SessionTerminatedNotification;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use tracing::trace;

pub struct TerminationListenerService<F: CommunicationFactory> {
    phantom: PhantomData<F>,
    handle: SessionCreationCommunicationHandle,
}

impl<F> Service<F> for TerminationListenerService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "FailureListenerService";
    type Instance = TerminationListenerService<F>;
    type Config = SessionCreationCommunicationHandle;

    fn instantiate(_factory: F, handle: &Self::Config) -> Self::Instance {
        Self {
            phantom: PhantomData,
            handle: handle.clone(),
        }
    }
}

#[async_trait]
impl<F> Consumer for TerminationListenerService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    type Notification = SessionTerminatedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        if let Some(tx) = self
            .handle
            .status_listeners
            .lock()
            .await
            .pop(&notification.id)
        {
            trace!(id = ?notification.id, "Forwarding terminated notification");
            tx.send(StatusResponse::Failed(notification.into_inner()))
                .ok();
        }

        Ok(())
    }
}
