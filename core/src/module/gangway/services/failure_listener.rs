use std::marker::PhantomData;

use super::super::{SessionCreationCommunicationHandle, StatusResponse};
use crate::domain::event::SessionStartupFailedNotification;
use crate::harness::Service;
use crate::library::communication::event::{Consumer, NotificationFrame};
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;

pub struct FailureListenerService<F: CommunicationFactory> {
    phantom: PhantomData<F>,
    handle: SessionCreationCommunicationHandle,
}

impl<F> Service<F> for FailureListenerService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "FailureListenerService";
    type Instance = FailureListenerService<F>;
    type Config = SessionCreationCommunicationHandle;

    fn instantiate(_factory: F, handle: &Self::Config) -> Self::Instance {
        Self {
            phantom: PhantomData,
            handle: handle.clone(),
        }
    }
}

#[async_trait]
impl<F> Consumer for FailureListenerService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    type Notification = SessionStartupFailedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        if let Some(tx) = self
            .handle
            .status_listeners
            .lock()
            .await
            .pop(&notification.id)
        {
            if tx
                .send(StatusResponse::Failed(notification.into_inner()))
                .is_err()
            {
                log::error!("Failed to send StatusResponse: receiver has been dropped");
            }
        }

        Ok(())
    }
}
