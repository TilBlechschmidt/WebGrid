use std::sync::Arc;

use super::MatchingStrategy;
use crate::domain::event::ProvisionerIdentifier;
use crate::domain::request::{ProvisionerMatchRequest, ProvisionerMatchResponse};
use crate::harness::Service;
use crate::library::communication::request::{OptionalRequestProcessor, Request, Responder};
use crate::library::communication::CommunicationFactory;
use crate::library::BoxedError;
use async_trait::async_trait;

/// Matches a provisioner using a [`MatchingStrategy`]
///
/// Consumes:
/// - [`ProvisionerMatchRequest`]
///
/// Responds with:
/// - [`ProvisionerMatchResponse`]
pub struct ProvisionerMatchingService<M: MatchingStrategy> {
    strategy: Arc<M>,
    provisioner: ProvisionerIdentifier,
}

impl<F, M> Service<F> for ProvisionerMatchingService<M>
where
    F: CommunicationFactory + Send + Sync,
    M: MatchingStrategy + Send + Sync,
{
    const NAME: &'static str = "ProvisionerMatchingService";
    type Instance = Responder<
        ProvisionerMatchRequest,
        ProvisionerMatchingService<M>,
        <F as CommunicationFactory>::ResponsePublisher,
    >;

    type Config = (ProvisionerIdentifier, Arc<M>);

    fn instantiate(factory: F, config: &Self::Config) -> Self::Instance {
        let publisher = factory.response_publisher();
        let processor = Self {
            provisioner: config.0.clone(),
            strategy: config.1.clone(),
        };

        Responder::new(processor, publisher)
    }
}

#[async_trait]
impl<M> OptionalRequestProcessor for ProvisionerMatchingService<M>
where
    M: MatchingStrategy + Send + Sync,
{
    type Request = ProvisionerMatchRequest;

    async fn maybe_process(
        &self,
        request: Self::Request,
    ) -> Result<Option<<Self::Request as Request>::Response>, BoxedError> {
        let response = if self.strategy.matches(request.capabilities) {
            Some(ProvisionerMatchResponse {
                provisioner: self.provisioner.clone(),
            })
        } else {
            None
        };

        Ok(response)
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use crate::domain::webdriver::{Capabilities, CapabilitiesRequest};

    impl<M> ProvisionerMatchingService<M>
    where
        M: MatchingStrategy + Send + Sync,
    {
        /// Creates a new instance for a given provisioner with a given strategy
        pub fn new(provisioner: ProvisionerIdentifier, strategy: M) -> Self {
            Self {
                strategy: Arc::new(strategy),
                provisioner,
            }
        }
    }

    struct BooleanMatchingStrategy(bool);

    impl MatchingStrategy for BooleanMatchingStrategy {
        fn matches(&self, _: CapabilitiesRequest) -> bool {
            self.0
        }
    }

    struct EqMatchingStrategy(CapabilitiesRequest);

    impl MatchingStrategy for EqMatchingStrategy {
        fn matches(&self, request: CapabilitiesRequest) -> bool {
            self.0 == request
        }
    }

    #[tokio::test]
    async fn ignore_non_matching_request() {
        let strategy = BooleanMatchingStrategy(false);
        let processor = ProvisionerMatchingService::new("some-id".into(), strategy);
        let request = ProvisionerMatchRequest::new(CapabilitiesRequest {
            first_match: None,
            always_match: None,
        });

        assert!(processor.maybe_process(request).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reply_to_matching_request() {
        let provisioner: String = "some-id".into();
        let strategy = BooleanMatchingStrategy(true);
        let processor = ProvisionerMatchingService::new(provisioner.clone(), strategy);
        let request = ProvisionerMatchRequest::new(CapabilitiesRequest {
            first_match: None,
            always_match: None,
        });

        assert_eq!(
            processor.maybe_process(request).await.unwrap(),
            Some(ProvisionerMatchResponse { provisioner })
        );
    }

    #[tokio::test]
    async fn forward_expected_request_to_strategy() {
        let capabilities = CapabilitiesRequest {
            first_match: Some(vec![Capabilities::empty()]),
            always_match: None,
        };

        let provisioner: String = "some-id".into();
        let strategy = EqMatchingStrategy(capabilities.clone());
        let processor = ProvisionerMatchingService::new(provisioner.clone(), strategy);
        let request = ProvisionerMatchRequest::new(capabilities);

        assert_eq!(
            processor.maybe_process(request).await.unwrap(),
            Some(ProvisionerMatchResponse { provisioner })
        );
    }
}
