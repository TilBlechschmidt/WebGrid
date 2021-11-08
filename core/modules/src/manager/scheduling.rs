use async_trait::async_trait;
use domain::event::{
    ProvisionerIdentifier, ProvisioningJobAssignedNotification, SessionCreatedNotification,
    SessionMetadataModifiedNotification, SessionScheduledNotification,
    SessionTerminatedNotification,
};
use domain::request::ProvisionerMatchRequest;
use harness::Service;
use library::communication::event::{Consumer, NotificationFrame, NotificationPublisher};
use library::communication::request::{RequestError, Requestor, ResponseCollectionTimeout};
use library::communication::{BlackboxError, CommunicationFactory};
use library::EmptyResult;
use rand::{seq::SliceRandom, thread_rng};
use std::collections::HashSet;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, instrument, warn};

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

    #[error("notification publishing failed")]
    PublishFailure(#[from] BlackboxError),

    #[error("required metadata fields were not provided: {0}")]
    MissingMandatoryMetadata(String),
}

/// Assigns a provisioner to a session
///
/// Consumes:
/// - [`SessionCreatedNotification`]
///
/// Publishes:
/// - [`SessionTerminatedNotification`]
/// - [`ProvisioningJobAssignedNotification`]
/// - [`SessionScheduledNotification`]
///
/// Requests:
/// - [`ProvisionerMatchRequest`]
pub struct SchedulingService<F: CommunicationFactory> {
    publisher: <F as CommunicationFactory>::NotificationPublisher,
    requestor: <F as CommunicationFactory>::Requestor,
    required_metadata: HashSet<String>,
}

impl<F> Service<F> for SchedulingService<F>
where
    F: CommunicationFactory + Send + Sync,
{
    const NAME: &'static str = "SchedulingService";
    type Instance = SchedulingService<F>;
    type Config = HashSet<String>;

    fn instantiate(factory: F, config: &Self::Config) -> Self::Instance {
        Self {
            publisher: factory.notification_publisher(),
            requestor: factory.requestor(),
            required_metadata: config.clone(),
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

        // Emit metadata contained in requested capabilities
        if let Some(metadata) = capabilities
            .clone()
            .into_sets()
            .into_iter()
            .find_map(|c| c.webgrid_options.map(|o| o.metadata).flatten())
        {
            let missing_keys = self
                .required_metadata
                .iter()
                .filter(|key| !metadata.contains_key(*key))
                .map(String::clone)
                .collect::<Vec<_>>();

            if !missing_keys.is_empty() {
                return Err(SchedulingServiceError::MissingMandatoryMetadata(
                    missing_keys.join(","),
                ));
            }

            debug!(count = metadata.len(), "Publishing metadata values");

            let notification = SessionMetadataModifiedNotification {
                id: notification.id,
                metadata,
            };

            self.publisher
                .publish(&notification)
                .await
                .map_err(BlackboxError::from_boxed)?;
        } else if !self.required_metadata.is_empty() {
            debug!("Required metadata is missing");
            return Err(SchedulingServiceError::MissingMandatoryMetadata(
                self.required_metadata
                    .clone()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }

        // Ask around for provisioners that can handle the requirements
        debug!("Requesting available provisioners");
        let request = ProvisionerMatchRequest::new(capabilities);
        let mut responses = self
            .requestor
            .request(&request, None, MATCHING_TIMEOUT)
            .await?;

        // Provide some laymans load balancing until load factors are implemented
        debug!(count = responses.len(), "Received available provisioners");
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

    #[instrument(skip(self, notification), fields(id = ?notification.id))]
    async fn consume(&self, notification: NotificationFrame<Self::Notification>) -> EmptyResult {
        debug!("Handling session creation");

        match self.handle_event(&notification).await {
            Err(SchedulingServiceError::RequestFailure(e)) => {
                warn!(error = ?e, "Session scheduling failed");
                Err(e.into())
            }
            Err(e) => {
                // Tell everybody that we have failed them :(
                warn!(error = ?e, "Session scheduling failed");
                let notification = SessionTerminatedNotification::new_for_startup_failure(
                    notification.id,
                    BlackboxError::new(e),
                );

                self.publisher.publish(&notification).await
            }
            Ok(provisioner) => {
                info!(?provisioner, "Scheduled session");

                // Let the provisioner know it has got a new job
                let provisioner_queue_extension = provisioner.to_string();
                let job_assignment_notification = ProvisioningJobAssignedNotification {
                    session_id: notification.id,
                    capabilities: notification.capabilities.to_owned(),
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
    use std::collections::HashMap;

    use super::*;
    use domain::request::ProvisionerMatchResponse;
    use domain::webdriver::{
        Capabilities, CapabilitiesRequest, RawCapabilitiesRequest, WebGridOptions,
    };
    use lazy_static::lazy_static;
    use library::communication::implementation::mock::MockCommunicationFactory;
    use uuid::Uuid;

    lazy_static! {
        static ref SESSION_ID: Uuid = Uuid::new_v4();
        static ref PROVISIONER_ID: String = "some-id".into();
    }

    #[tokio::test]
    async fn publish_expected_notifications() {
        let raw_capabilities = CapabilitiesRequest::default();
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

        SchedulingService::instantiate(factory, &HashSet::new())
            .consume(NotificationFrame::new(created))
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

        let failure = SessionTerminatedNotification::new_for_startup_failure(*SESSION_ID, cause);

        let factory = MockCommunicationFactory::default();
        factory.expect(&failure);

        SchedulingService::instantiate(factory, &HashSet::new())
            .consume(NotificationFrame::new(created))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rejects_missing_metadata_when_required_fields_are_provided() {
        let mut required_metadata = HashSet::new();
        required_metadata.insert("project".into());

        let raw_capabilities = CapabilitiesRequest::default();
        let capabilities =
            RawCapabilitiesRequest::new(serde_json::to_string(&raw_capabilities).unwrap());

        let created = SessionCreatedNotification {
            id: *SESSION_ID,
            capabilities: capabilities.clone(),
        };

        let cause = BlackboxError::new(SchedulingServiceError::MissingMandatoryMetadata(
            "project".into(),
        ));
        let failure = SessionTerminatedNotification::new_for_startup_failure(*SESSION_ID, cause);

        let factory = MockCommunicationFactory::default();
        factory.expect(&failure);

        SchedulingService::instantiate(factory, &required_metadata)
            .consume(NotificationFrame::new(created))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rejects_incomplete_metadata_when_required_fields_are_provided() {
        let mut required_metadata = HashSet::new();
        required_metadata.insert("project".into());

        let mut metadata = HashMap::new();
        metadata.insert("pipeline".into(), "1337".into());

        let raw_capabilities = CapabilitiesRequest {
            always_match: Some(Capabilities {
                webgrid_options: Some(WebGridOptions {
                    metadata: Some(metadata.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let capabilities =
            RawCapabilitiesRequest::new(serde_json::to_string(&raw_capabilities).unwrap());

        let created = SessionCreatedNotification {
            id: *SESSION_ID,
            capabilities: capabilities.clone(),
        };

        let cause = BlackboxError::new(SchedulingServiceError::MissingMandatoryMetadata(
            "project".into(),
        ));
        let failure = SessionTerminatedNotification::new_for_startup_failure(*SESSION_ID, cause);

        let factory = MockCommunicationFactory::default();
        factory.expect(&failure);

        SchedulingService::instantiate(factory, &required_metadata)
            .consume(NotificationFrame::new(created))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn accept_metadata_when_required_fields_are_provided() {
        let mut required_metadata = HashSet::new();
        required_metadata.insert("project".into());

        let mut metadata = HashMap::new();
        metadata.insert("project".into(), "webgrid".into());

        let raw_capabilities = CapabilitiesRequest {
            always_match: Some(Capabilities {
                webgrid_options: Some(WebGridOptions {
                    metadata: Some(metadata.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
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

        let metadata_modified = SessionMetadataModifiedNotification {
            id: *SESSION_ID,
            metadata,
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
            .expect(&metadata_modified)
            .expect_with_extension(&job_assigned, PROVISIONER_ID.to_string())
            .expect(&scheduled);

        SchedulingService::instantiate(factory, &required_metadata)
            .consume(NotificationFrame::new(created))
            .await
            .unwrap();
    }
}
