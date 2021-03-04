use super::super::super::core::provisioner::{
    match_image_from_capabilities, NodeInfo, Provisioner, ProvisionerCapabilities,
};
use crate::libraries::helpers::CapabilitiesRequest;
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, KillContainerOptions, StartContainerOptions},
    models::HostConfig,
    Docker,
};
use log::{debug, warn};
use std::default::Default;

#[derive(Clone)]
pub struct DockerProvisioner {
    docker: Docker,
    images: Vec<(String, String)>,
    node_port: u16,
}

impl DockerProvisioner {
    pub fn new(node_port: u16, images: Vec<(String, String)>) -> Self {
        if images.is_empty() {
            warn!("No images provided! Orchestrator won't be able to schedule nodes.");
        }

        // TODO Remove unwrap
        let connection = Docker::connect_with_local_defaults().unwrap();

        Self {
            docker: connection,
            images,
            node_port,
        }
    }
}

#[async_trait]
impl Provisioner for DockerProvisioner {
    fn capabilities(&self) -> ProvisionerCapabilities {
        let browsers = self.images.iter().cloned().map(|i| i.1).collect();

        ProvisionerCapabilities {
            platform_name: "linux".to_owned(),
            browsers,
        }
    }

    async fn provision_node(
        &self,
        session_id: &str,
        capabilities: CapabilitiesRequest,
    ) -> NodeInfo {
        let wrapped_image = match_image_from_capabilities(capabilities, &self.images);
        // TODO Remove unwrap
        let image = wrapped_image.unwrap();

        let name = format!("webgrid-session-{}", session_id);

        let options = Some(CreateContainerOptions { name: &name });

        let env: Vec<String> = vec![
            "REDIS=redis://webgrid-redis/".to_string(),
            format!("ID={}", session_id),
            "STORAGE_DIRECTORY=/storage".to_string(),
            "RUST_LOG=trace,tokio=warn,hyper=warn,mio=warn,want=warn".to_string(),
        ];

        let host_config = HostConfig {
            auto_remove: Some(true),
            network_mode: Some("webgrid".to_string()),
            shm_size: Some(1024 * 1024 * 1024 * 2),
            binds: Some(vec!["webgrid:/storage".to_string()]),
            ..Default::default()
        };

        let config: Config<&str> = Config {
            image: Some(&image),
            hostname: Some(&name),
            host_config: Some(host_config),
            env: Some(env.iter().map(|e| e.as_ref()).collect()),
            ..Default::default()
        };

        debug!("Creating docker container {}", name);

        // TODO Remove unwrap
        self.docker.create_container(options, config).await.unwrap();
        self.docker
            .start_container(&name, None::<StartContainerOptions<String>>)
            .await
            .unwrap();

        NodeInfo {
            host: name,
            port: self.node_port.to_string(),
        }
    }

    async fn terminate_node(&self, session_id: &str) {
        let name = format!("webgrid-node-{}", session_id);
        debug!("Killing docker container {}", name);
        // TODO Handle potential errors.
        self.docker
            .kill_container(&name, None::<KillContainerOptions<String>>)
            .await
            .ok();
    }
}
