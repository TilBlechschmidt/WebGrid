use super::{super::provisioner::SessionProvisioner, ProvisioningState};
use async_trait::async_trait;
use domain::event::{
    ProvisionedSessionMetadata, ProvisioningJobAssignedNotification,
    SessionProvisionedNotification, SessionTerminatedNotification,
};
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame, NotificationPublisher};
use library::communication::request::RequestError;
use library::communication::{BlackboxError, CommunicationFactory};
use library::{BoxedError, EmptyResult};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::AcquireError;
use tracing::{debug, instrument};

#[derive(Debug, Error)]
enum ProvisioningServiceError {
    #[error("session provisioner failed")]
    ProvisioningFailed(#[source] BoxedError),

    #[error("unable to acquire permit")]
    NoPermit(#[from] AcquireError),

    #[error("provisioning request failed")]
    RequestFailure(#[from] RequestError),
}

/// Provisions new sessions using an underlying [`SessionProvisioner`]
pub struct ProvisioningService<S: SessionProvisioner, F: CommunicationFactory> {
    state: ProvisioningState,
    provisioner: Arc<S>,
    publisher: <F as CommunicationFactory>::NotificationPublisher,
}

impl<S, F> Service<F> for ProvisioningService<S, F>
where
    S: SessionProvisioner + Send + Sync,
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "ProvisioningService";
    type Instance = ProvisioningService<S, F>;
    type Config = (ProvisioningState, Arc<S>);

    fn instantiate(factory: F, config: &Self::Config) -> Self::Instance {
        Self {
            state: config.0.clone(),
            provisioner: config.1.clone(),
            publisher: factory.notification_publisher(),
        }
    }
}

impl<S, F> ProvisioningService<S, F>
where
    S: SessionProvisioner + Send + Sync,
    F: CommunicationFactory + Send + Sync,
{
    #[instrument(skip(self, notification), fields(id = ?notification.session_id))]
    async fn provision(
        &self,
        notification: &<Self as Consumer>::Notification,
    ) -> Result<ProvisionedSessionMetadata, ProvisioningServiceError> {
        // Get a permit so we don't deploy infinitely many sessions
        debug!("Acquiring permit");
        self.state.acquire_permit(notification.session_id).await?;

        // Provision the session
        debug!("Provisioning session");
        let meta = self
            .provisioner
            .provision(&notification.session_id, &notification.capabilities)
            .await
            .map_err(|e| ProvisioningServiceError::ProvisioningFailed(e))?;

        debug!("Provisioned session");
        Ok(meta)
    }
}

#[async_trait]
impl<S, F> Consumer for ProvisioningService<S, F>
where
    S: SessionProvisioner + Send + Sync,
    F: CommunicationFactory + Send + Sync,
{
    type Notification = ProvisioningJobAssignedNotification;

    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        match self.provision(&notification).await {
            Err(ProvisioningServiceError::RequestFailure(e)) => Err(e.into()),
            Err(e) => {
                // Tell everybody that we have failed them :(
                let terminated_notification =
                    SessionTerminatedNotification::new_for_startup_failure(
                        notification.session_id,
                        BlackboxError::new(e),
                    );

                self.publisher.publish(&terminated_notification).await
            }
            Ok(meta) => {
                // Notify everybody about our success
                let provisioned = SessionProvisionedNotification {
                    id: notification.session_id,
                    meta,
                };

                self.publisher.publish(&provisioned).await
            }
        }
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use domain::event::SessionIdentifier;
    use domain::webdriver::RawCapabilitiesRequest;
    use lazy_static::lazy_static;
    use library::communication::implementation::mock::MockCommunicationFactory;
    use thiserror::Error;
    use uuid::Uuid;

    lazy_static! {
        static ref SESSION_ID: Uuid = Uuid::new_v4();
    }

    #[derive(Debug, Error)]
    enum MockError {
        #[error("some error")]
        SomeError,
    }

    struct MockProvisioner<F>(F);

    impl<F> MockProvisioner<F>
    where
        F: Fn() -> Result<ProvisionedSessionMetadata, BoxedError>,
    {
        fn new(result: F) -> Arc<Self> {
            Arc::new(Self(result))
        }
    }

    #[async_trait]
    impl<F> SessionProvisioner for MockProvisioner<F>
    where
        F: Fn() -> Result<ProvisionedSessionMetadata, BoxedError> + Send + Sync,
    {
        async fn provision(
            &self,
            _session_id: &SessionIdentifier,
            _capabilities: &RawCapabilitiesRequest,
        ) -> Result<ProvisionedSessionMetadata, BoxedError> {
            (self.0)()
        }

        async fn alive_sessions(&self) -> Result<Vec<SessionIdentifier>, BoxedError> {
            unimplemented!()
        }

        async fn purge_terminated(&self) -> EmptyResult {
            unimplemented!()
        }
    }

    async fn run_with_provisioner<F>(
        provisioner: Arc<MockProvisioner<F>>,
        factory: impl CommunicationFactory + Send + Sync,
    ) where
        F: Fn() -> Result<ProvisionedSessionMetadata, BoxedError> + Send + Sync,
    {
        let state = ProvisioningState::new(1);
        let service = ProvisioningService::instantiate(factory, &(state, provisioner));
        let notification = ProvisioningJobAssignedNotification {
            session_id: *SESSION_ID,
            capabilities: RawCapabilitiesRequest::new("{}".into()),
        };

        service
            .consume(NotificationFrame::new(notification))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn publish_provisioned_notification() {
        let meta = ProvisionedSessionMetadata::new();
        let expected = SessionProvisionedNotification {
            id: *SESSION_ID,
            meta: meta.clone(),
        };

        let provisioner = MockProvisioner::new(|| Ok(meta.clone()));
        let factory = MockCommunicationFactory::default();
        factory.expect(&expected);

        run_with_provisioner(provisioner, factory).await;
    }

    #[tokio::test]
    async fn publish_startup_failure_notification() {
        let expected = SessionTerminatedNotification::new_for_startup_failure(
            *SESSION_ID,
            BlackboxError::new(ProvisioningServiceError::ProvisioningFailed(
                MockError::SomeError.into(),
            )),
        );

        let provisioner = MockProvisioner::new(|| Err(MockError::SomeError.into()));
        let factory = MockCommunicationFactory::default();
        factory.expect(&expected);

        run_with_provisioner(provisioner, factory).await;
    }
}
