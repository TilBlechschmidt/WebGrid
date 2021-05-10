use super::super::super::core::provisioner::{
    match_image_from_capabilities, NodeInfo, Provisioner, ProvisionerCapabilities,
};
use crate::libraries::helpers::CapabilitiesRequest;
use anyhow::{anyhow, Result};
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
    disable_recording: bool,
}

impl DockerProvisioner {
    pub fn new(
        node_port: u16,
        images: Vec<(String, String)>,
        disable_recording: bool,
    ) -> Result<Self> {
        if images.is_empty() {
            warn!("No images provided! Orchestrator won't be able to schedule nodes.");
        }

        let connection = Docker::connect_with_local_defaults()?;

        Ok(Self {
            docker: connection,
            images,
            node_port,
            disable_recording,
        })
    }
}

#[async_trait]
impl Provisioner for DockerProvisioner {
    fn capabilities(&self) -> ProvisionerCapabilities {
        ProvisionerCapabilities {
            platform_name: "linux".to_owned(),
            browsers: self
                .images
                .iter()
                .map(|(_, browser)| browser.to_owned())
                .collect(),
        }
    }

    async fn provision_node(
        &self,
        session_id: &str,
        capabilities: CapabilitiesRequest,
    ) -> Result<NodeInfo> {
        let image = match_image_from_capabilities(capabilities, &self.images)
            .ok_or_else(|| anyhow!("No matching image found!"))?;

        let name = format!("webgrid-session-{}", session_id);

        let options = Some(CreateContainerOptions { name: &name });

        let mut env: Vec<String> = vec![
            "REDIS=redis://webgrid-redis/".to_string(),
            format!("ID={}", session_id),
            "RUST_LOG=debug,hyper=warn,warp=warn,sqlx=warn,tower=warn,h2=warn".to_string(),
        ];

        if !self.disable_recording {
            env.push("STORAGE_DIRECTORY=/storage".to_string());
        }

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

        self.docker
            .create_container(options, config)
            .await
            .map_err(|e| {
                e
            })?;
        self.docker
            .start_container(&name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| {
                e
            })?;

        Ok(NodeInfo {
            host: name,
            port: self.node_port.to_string(),
        })
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
