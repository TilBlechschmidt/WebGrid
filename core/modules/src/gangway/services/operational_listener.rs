use super::super::{SessionCreationCommunicationHandle, StatusResponse};
use async_trait::async_trait;
use domain::event::SessionOperationalNotification;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame};
use library::communication::CommunicationFactory;
use library::EmptyResult;
use std::marker::PhantomData;
use tracing::trace;

pub struct OperationalListenerService<F: CommunicationFactory> {
    phantom: PhantomData<F>,
    handle: SessionCreationCommunicationHandle,
}

impl<F> Service<F> for OperationalListenerService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "OperationalListenerService";
    type Instance = OperationalListenerService<F>;
    type Config = SessionCreationCommunicationHandle;

    fn instantiate(_factory: F, handle: &Self::Config) -> Self::Instance {
        Self {
            phantom: PhantomData,
            handle: handle.clone(),
        }
    }
}

#[async_trait]
impl<F> Consumer for OperationalListenerService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    type Notification = SessionOperationalNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        if let Some(tx) = self
            .handle
            .status_listeners
            .lock()
            .await
            .pop(&notification.id)
        {
            trace!(id = ?notification.id, "Forwarding operational notification");
            tx.send(StatusResponse::Operational(notification.into_inner()))
                .ok();
        }

        Ok(())
    }
}
