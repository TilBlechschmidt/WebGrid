use std::collections::HashMap;
use std::str::FromStr;

use super::SessionProvisioner;
use super::{CONTAINER_SESSION_ID_LABEL, PROVISIONER_INSTANCE_LABEL};
use crate::domain::container::ContainerImageSet;
use crate::domain::event::{ProvisionedSessionMetadata, SessionIdentifier};
use crate::domain::webdriver::RawCapabilitiesRequest;
use crate::library::{BoxedError, EmptyResult};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use futures::StreamExt;
use thiserror::Error;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

#[derive(Error, Debug)]
enum DockerProvisionerError {
    #[error("no matching image found")]
    NoImageFound,

    #[error("image pull failed")]
    ImagePullError(#[source] bollard::errors::Error),

    #[error("create docker container failed")]
    CreateContainerError(#[source] bollard::errors::Error),

    #[error("start docker container failed")]
    StartContainerError(#[source] bollard::errors::Error),

    #[error("invalid capabilities")]
    InvalidCapabilities(#[from] serde_json::Error),
}

/// Implementation based on [Docker](https://www.docker.com/) containers
pub struct DockerProvisioner {
    docker: Docker,
    images: ContainerImageSet,
    instance: Uuid,
    auto_remove: bool,
    storage: Option<String>,
    log: String,
}

impl DockerProvisioner {
    /// Creates a new instance with the provided images, connecting to the default docker instance
    pub fn new(
        images: ContainerImageSet,
        auto_remove: bool,
        storage: Option<String>,
        log: String,
    ) -> Result<Self, bollard::errors::Error> {
        if images.is_empty() {
            warn!("No images provided to provisioner. It won't be able to launch any sessions!");
        }

        let connection = Docker::connect_with_local_defaults()?;
        let instance = Uuid::new_v4();

        Ok(Self {
            docker: connection,
            images,
            instance,
            auto_remove,
            storage,
            log,
        })
    }

    #[instrument(err, skip(self))]
    async fn pull_image(&self, image: &str) -> Result<(), bollard::errors::Error> {
        let options = Some(CreateImageOptions {
            from_image: image,
            ..Default::default()
        });

        // Check if the image is available locally
        if self.docker.inspect_image(image).await.is_ok() {
            debug!("Image available locally");
            return Ok(());
        }

        // Attempt to pull the requested image
        let mut stream = self.docker.create_image(options, None, None);

        while let Some(result) = stream.next().await {
            result?;
        }

        Ok(())
    }

    #[instrument(err, skip(self, raw_capabilities))]
    async fn create_container(
        &self,
        session_id: &SessionIdentifier,
        raw_capabilities: &RawCapabilitiesRequest,
    ) -> Result<ProvisionedSessionMetadata, DockerProvisionerError> {
        let request = raw_capabilities.parse()?;
        let image = self
            .images
            .match_against_capabilities(request)
            .ok_or(DockerProvisionerError::NoImageFound)?;

        debug!("Pulling image {:?}", image);
        self.pull_image(&image.identifier)
            .await
            .map_err(DockerProvisionerError::ImagePullError)?;

        let name = format!("webgrid-session-{}", session_id);
        let mut env: Vec<String> = vec![
            format!("ID={}", session_id),
            format!("CAPABILITIES={}", raw_capabilities.as_str()),
            format!("HOST={}", name.as_str()),
            format!("RUST_LOG={}", self.log),
        ];

        if let Some(storage) = &self.storage {
            env.push(format!("STORAGE={}", storage));
        }

        let mut labels = HashMap::<&str, &str>::new();
        let instance_id = self.instance.to_string();
        let session_id_label = session_id.to_string();
        labels.insert(PROVISIONER_INSTANCE_LABEL, &instance_id);
        labels.insert(CONTAINER_SESSION_ID_LABEL, &session_id_label);

        let options = Some(CreateContainerOptions { name: &name });

        let host_config = HostConfig {
            auto_remove: Some(self.auto_remove),
            network_mode: Some("webgrid".to_string()),
            shm_size: Some(1024 * 1024 * 1024 * 2),
            ..Default::default()
        };

        let config: Config<&str> = Config {
            image: Some(&image.identifier),
            hostname: Some(&name),
            host_config: Some(host_config),
            env: Some(env.iter().map(|e| e.as_ref()).collect()),
            labels: Some(labels),
            ..Default::default()
        };

        debug!(?name, "Creating docker container");
        self.docker
            .create_container(options, config)
            .await
            .map_err(DockerProvisionerError::CreateContainerError)?;

        debug!(?name, "Starting docker container");
        self.docker
            .start_container(&name, None::<StartContainerOptions<String>>)
            .await
            .map_err(DockerProvisionerError::StartContainerError)?;

        // TODO Append more meaningful information
        Ok(ProvisionedSessionMetadata::new())
    }

    async fn list_running_containers(
        &self,
    ) -> Result<Vec<SessionIdentifier>, bollard::errors::Error> {
        let instance_label_filter = format!("{}={}", PROVISIONER_INSTANCE_LABEL, self.instance);

        let mut filters = HashMap::<&str, Vec<&str>>::new();
        filters.insert("label", vec![&instance_label_filter]);

        let options = ListContainersOptions {
            filters,
            ..Default::default()
        };

        Ok(self
            .docker
            .list_containers(Some(options))
            .await?
            .into_iter()
            .filter_map(|container| match container.labels {
                None => None,
                Some(labels) => labels
                    .get(CONTAINER_SESSION_ID_LABEL)
                    .map(|id| {
                        Uuid::from_str(id)
                            .map_err(|e| {
                                warn!(?id, ?e, "Failed to parse session id from container label",)
                            })
                            .ok()
                    })
                    .flatten(),
            })
            .collect())
    }
}

#[async_trait]
impl SessionProvisioner for DockerProvisioner {
    async fn provision(
        &self,
        session_id: &SessionIdentifier,
        raw_capabilities: &RawCapabilitiesRequest,
    ) -> Result<ProvisionedSessionMetadata, BoxedError> {
        Ok(self.create_container(session_id, raw_capabilities).await?)
    }

    async fn alive_sessions(&self) -> Result<Vec<SessionIdentifier>, BoxedError> {
        Ok(self.list_running_containers().await?)
    }

    /// In Docker this is handled automagically by the auto_remove property of the [`HostConfig`]
    async fn purge_terminated(&self) -> EmptyResult {
        Ok(())
    }
}

// TODO Write tests for the docker provisioner (using a dummy image and checking with the API)
