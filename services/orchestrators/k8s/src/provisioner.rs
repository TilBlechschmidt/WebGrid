
use orchestrator_core::provisioner::{Provisioner, NodeInfo, async_trait};

pub struct K8sProvisioner {}

impl K8sProvisioner {
    pub async fn new() -> Self {
        Self {
        }
    }
}

#[async_trait]
impl Provisioner for K8sProvisioner {
    async fn provision_node(&self, session_id: &str) -> NodeInfo {
        let name = format!("webgrid-node-{}", session_id);

        NodeInfo {
            host: name,
            port: "3030".to_string(),
        }
    }
    
    async fn terminate_node(&self, session_id: &str) {
        let _name = format!("webgrid-node-{}", session_id);
    }
}