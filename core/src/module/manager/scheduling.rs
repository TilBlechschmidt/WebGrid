use crate::domain::event::{
    ProvisionerIdentifier, ProvisioningJobAssignedNotification, SessionCreatedNotification,
    SessionScheduledNotification, SessionStartupFailedNotification,
};
use crate::domain::request::ProvisionerMatchRequest;
use crate::harness::Service;
use crate::library::communication::event::{Consumer, NotificationPublisher};
use crate::library::communication::request::{RequestError, Requestor, ResponseCollectionTimeout};
use crate::library::communication::{BlackboxError, CommunicationFactory};
use crate::library::EmptyResult;
use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use std::time::Duration;
use thiserror::Error;

const MATCHING_TIMEOUT: ResponseCollectionTimeout =
    ResponseCollectionTimeout::Split(Duration::from_secs(10), Duration::from_millis(100));

#[derive(Debug, Error)]
enum SchedulingServiceError {
    #[error("capabilities parsing failed")]
    ParsingFailed(#[from] serde_json::Error),

    #[error("no provisioner answered the matching request")]
    NoProvisioner,

    #[error("matching request failed")]
    RequestFailure(#[from] RequestError),
}

/// Assigns a provisioner to a session
///
/// Consumes:
/// - [`SessionCreatedNotification`]
///
/// Publishes:
/// - [`SessionStartupFailedNotification`]
/// - [`ProvisioningJobAssignedNotification`]
/// - [`SessionScheduledNotification`]
///
/// Requests:
/// - [`ProvisionerMatchRequest`]
pub struct SchedulingService<F: CommunicationFactory> {
    publisher: <F as CommunicationFactory>::NotificationPublisher,
    requestor: <F as CommunicationFactory>::Requestor,
}

impl<F> Service<F> for SchedulingService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "SchedulingService";
    type Instance = SchedulingService<F>;
    type Config = ();

    fn instantiate(factory: F, _config: &Self::Config) -> Self::Instance {
        Self {
            publisher: factory.notification_publisher(),
            requestor: factory.requestor(),
        }
    }
}

impl<F> SchedulingService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    async fn handle_event(
        &self,
        notification: &<Self as Consumer>::Notification,
    ) -> Result<ProvisionerIdentifier, SchedulingServiceError> {
        let capabilities = notification.capabilities.parse()?;
        let request = ProvisionerMatchRequest::new(capabilities);

        let mut responses = self
            .requestor
            .request(&request, None, MATCHING_TIMEOUT)
            .await?;

        // Provide some laymans load balancing until load factors are implemented
        responses.shuffle(&mut thread_rng());

        responses
            .pop()
            .ok_or(SchedulingServiceError::NoProvisioner)
            .map(|r| r.provisioner)
    }
}

#[async_trait]
impl<F> Consumer for SchedulingService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    type Notification = SessionCreatedNotification;

    async fn consume(&self, notification: Self::Notification) -> EmptyResult {
        match self.handle_event(&notification).await {
            Err(SchedulingServiceError::RequestFailure(e)) => Err(e.into()),
            Err(e) => {
                // Tell everybody that we have failed them :(
                let failure_notification = SessionStartupFailedNotification {
                    id: notification.id,
                    cause: BlackboxError::new(e),
                };

                self.publisher.publish(&failure_notification).await
            }
            Ok(provisioner) => {
                // Let the provisioner know it has got a new job
                let provisioner_queue_extension = provisioner.to_string();
                let job_assignment_notification = ProvisioningJobAssignedNotification {
                    session_id: notification.id,
                    capabilities: notification.capabilities,
                };

                self.publisher
                    .publish_with_extension(
                        &job_assignment_notification,
                        provisioner_queue_extension,
                    )
                    .await?;

                // Publish the notification for the scheduled event
                let scheduled_notification = SessionScheduledNotification {
                    id: notification.id,
                    provisioner,
                };

                self.publisher.publish(&scheduled_notification).await
            }
        }
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use crate::domain::request::ProvisionerMatchResponse;
    use crate::domain::webdriver::{CapabilitiesRequest, RawCapabilitiesRequest};
    use crate::library::communication::implementation::mock::MockCommunicationFactory;
    use lazy_static::lazy_static;
    use uuid::Uuid;

    lazy_static! {
        static ref SESSION_ID: Uuid = Uuid::new_v4();
        static ref PROVISIONER_ID: String = "some-id".into();
    }

    #[tokio::test]
    async fn publish_expected_notifications() {
        let raw_capabilities = CapabilitiesRequest {
            first_match: None,
            always_match: None,
        };
        let capabilities =
            RawCapabilitiesRequest::new(serde_json::to_string(&raw_capabilities).unwrap());

        let created = SessionCreatedNotification {
            id: *SESSION_ID,
            capabilities: capabilities.clone(),
        };

        let scheduled = SessionScheduledNotification {
            id: *SESSION_ID,
            provisioner: (*PROVISIONER_ID).clone(),
        };

        let job_assigned = ProvisioningJobAssignedNotification {
            session_id: *SESSION_ID,
            capabilities: capabilities.clone(),
        };

        let match_request = ProvisionerMatchRequest::new(raw_capabilities);
        let match_response = vec![ProvisionerMatchResponse {
            provisioner: (*PROVISIONER_ID).clone(),
        }];

        let factory = MockCommunicationFactory::default();

        factory
            .expect_and_respond(&match_request, match_response)
            .expect_with_extension(&job_assigned, PROVISIONER_ID.to_string())
            .expect(&scheduled);

        SchedulingService::instantiate(factory, &())
            .consume(created)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn fail_softly_on_invalid_capabilities() {
        let capabilities = RawCapabilitiesRequest::new("definitely-not-valid-capabilities".into());

        let error = capabilities.parse().unwrap_err();
        let cause = BlackboxError::new(SchedulingServiceError::ParsingFailed(error));

        let created = SessionCreatedNotification {
            id: *SESSION_ID,
            capabilities,
        };

        let failure = SessionStartupFailedNotification {
            id: *SESSION_ID,
            cause,
        };

        let factory = MockCommunicationFactory::default();
        factory.expect(&failure);

        SchedulingService::instantiate(factory, &())
            .consume(created)
            .await
            .unwrap();
    }
}
