//! Module to provision new instances of the [`node`](super::node) module

use std::sync::Arc;

use self::provisioner::{DockerProvisioner, KubernetesProvisioner, SessionProvisioner};
use crate::harness::{Heart, Module, ServiceRunner};
use crate::library::communication::event::ConsumerGroupDescriptor;
use crate::library::BoxedError;
use async_trait::async_trait;
use jatsl::{schedule, JobScheduler};
use options::{OrchestratorOptions, ProvisionerCommand};
use services::*;

mod options;
mod provisioner;
mod services;

pub use options::Options;

type BoxedProvisioner = Arc<Box<dyn SessionProvisioner + Send + Sync>>;
type BoxedMatchingStrategy = Arc<Box<dyn MatchingStrategy + Send + Sync>>;

/// Module implementation
pub struct Orchestrator {
    options: OrchestratorOptions,
    provisioner: BoxedProvisioner,
    matching_strategy: BoxedMatchingStrategy,
}

impl Orchestrator {
    /// Creates a new instance from raw parts
    pub fn new(command: Options) -> Self {
        let (options, provisioner, matching_strategy): (
            OrchestratorOptions,
            BoxedProvisioner,
            BoxedMatchingStrategy,
        ) = match command.provisioner {
            ProvisionerCommand::Kubernetes(provisioner_options) => {
                let provisioner = KubernetesProvisioner::new(provisioner_options.images.clone());

                (
                    provisioner_options.orchestrator,
                    Arc::new(Box::new(provisioner)),
                    Arc::new(Box::new(ContainerMatchingStrategy::new(
                        provisioner_options.images,
                    ))),
                )
            }
            ProvisionerCommand::Docker(provisioner_options) => {
                let provisioner = DockerProvisioner::new(
                    provisioner_options.images.clone(),
                    !provisioner_options.retain_exited_sessions,
                )
                .unwrap();

                (
                    provisioner_options.orchestrator,
                    Arc::new(Box::new(provisioner)),
                    Arc::new(Box::new(ContainerMatchingStrategy::new(
                        provisioner_options.images,
                    ))),
                )
            }
        };

        Self {
            options,
            provisioner,
            matching_strategy,
        }
    }
}

#[async_trait]
impl Module for Orchestrator {
    async fn run(&mut self, scheduler: &JobScheduler) -> Result<Option<Heart>, BoxedError> {
        let redis_url = &self.options.redis.url;
        let state = ProvisioningState::new(self.options.permits);

        let matching_service = ServiceRunner::<ProvisionerMatchingService<_>>::new(
            redis_url.clone(),
            ConsumerGroupDescriptor::default(),
            self.options.queueing.id.to_string(),
            (
                self.options.queueing.id.clone(),
                self.matching_strategy.clone(),
            ),
        );

        let provisioning_extension = Some(self.options.queueing.id.to_string());
        let provisioning_service = ServiceRunner::<ProvisioningService<_, _>>::new_with_extension(
            redis_url.clone(),
            provisioning_extension,
            ConsumerGroupDescriptor::default(),
            self.options.queueing.id.to_string(),
            (state.clone(), self.provisioner.clone()),
        );

        let termination_service = ServiceRunner::<SessionTerminationWatcherService>::new(
            redis_url.clone(),
            ConsumerGroupDescriptor::default(),
            self.options.queueing.id.to_string(),
            state.clone(),
        );

        let sync_service = HardwareSynchronisationService::new(
            state,
            self.provisioner.clone(),
            self.options.cleanup_interval,
        );

        schedule!(scheduler, {
            matching_service,
            provisioning_service,
            termination_service,
            sync_service,
        });

        Ok(Some(Heart::without_heart_stone()))
    }
}
