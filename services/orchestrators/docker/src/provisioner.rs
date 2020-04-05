
use orchestrator_core::provisioner::{Provisioner, NodeInfo, async_trait};

use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, HostConfig, KillContainerOptions, StartContainerOptions,
};

use std::default::Default;

#[derive(Clone)]
pub struct DockerProvisioner {
    docker: Docker,
}

impl DockerProvisioner {
    pub fn new() -> Self {
        // TODO Remove unwrap
        let connection = Docker::connect_with_local_defaults().unwrap();

        Self { docker: connection }
    }
}

#[async_trait]
impl Provisioner for DockerProvisioner {
    async fn provision_node(&self, session_id: &str) -> NodeInfo {
        let name = format!("webgrid-node-{}", session_id);

        let options = Some(CreateContainerOptions { name: &name });

        let env: Vec<String> = vec![
            format!("WEBGRID_REDIS_URL=redis://webgrid-redis/"),
            format!("WEBGRID_SESSION_ID={}", session_id),
            format!("FFMPEG_LOG=/host/{}-ffmpeg.log", session_id),
            format!("FFMPEG_OUT=/host/{}-ffmpeg.mp4", session_id),
        ];

        let host_config = HostConfig {
            auto_remove: Some(true),
            network_mode: Some("webgrid"),
            shm_size: Some(1024 * 1024 * 1024 * 2),
            binds: Some(vec!["/tmp/vr:/host"]),
            ..Default::default()
        };

        let config = Config {
            image: Some("webgrid-node:firefox"),
            hostname: Some(&name),
            host_config: Some(host_config),
            env: Some(env.iter().map(|e| e.as_ref()).collect()),
            ..Default::default()
        };

        // TODO Remove unwrap
        self.docker.create_container(options, config).await.unwrap();
        self.docker
            .start_container(&name, None::<StartContainerOptions<String>>)
            .await
            .unwrap();

        NodeInfo {
            host: name,
            port: "3030".to_string(),
        }
    }
    
    async fn terminate_node(&self, session_id: &str) {
        let name = format!("webgrid-node-{}", session_id);
        // TODO Remove unwrap
        self.docker
            .kill_container(&name, None::<KillContainerOptions<String>>)
            .await
            .unwrap();
    }
}