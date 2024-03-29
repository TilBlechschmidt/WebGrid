//! Glue layers for various infrastructure providers

use async_trait::async_trait;
use domain::event::{ProvisionedSessionMetadata, SessionIdentifier};
use domain::webdriver::RawCapabilitiesRequest;
use library::{BoxedError, EmptyResult};

mod docker;
mod kubernetes;

pub use docker::DockerProvisioner;
pub use kubernetes::KubernetesProvisioner;

/// Label defining the instance which manages the container
pub const PROVISIONER_INSTANCE_LABEL: &str = "dev.webgrid/provisioner.instance";
/// Label defining the session id the container is bound to
pub const CONTAINER_SESSION_ID_LABEL: &str = "dev.webgrid/session.id";

/// Intermediary providing indirect access to hardware on which sessions can run
#[async_trait]
pub trait SessionProvisioner {
    /// Dispatches a new session with the provided identifier and capabilities
    async fn provision(
        &self,
        session_id: &SessionIdentifier,
        capabilities: &RawCapabilitiesRequest,
    ) -> Result<ProvisionedSessionMetadata, BoxedError>;

    /// Retrieves a list of active sessions on the hardware that have been provisioned by this instance
    async fn alive_sessions(&self) -> Result<Vec<SessionIdentifier>, BoxedError>;

    /// Instructs the provisioner to purge orphaned or dead resources
    async fn purge_terminated(&self) -> EmptyResult;
}

#[async_trait]
impl<'a> SessionProvisioner for Box<dyn SessionProvisioner + Send + Sync + 'a> {
    async fn provision(
        &self,
        session_id: &domain::event::SessionIdentifier,
        capabilities: &domain::webdriver::RawCapabilitiesRequest,
    ) -> Result<domain::event::ProvisionedSessionMetadata, library::BoxedError> {
        self.as_ref().provision(session_id, capabilities).await
    }

    async fn alive_sessions(
        &self,
    ) -> Result<Vec<domain::event::SessionIdentifier>, library::BoxedError> {
        self.as_ref().alive_sessions().await
    }

    async fn purge_terminated(&self) -> library::EmptyResult {
        self.as_ref().purge_terminated().await
    }
}
