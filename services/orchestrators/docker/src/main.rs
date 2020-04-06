mod provisioner;

use crate::provisioner::DockerProvisioner;
use orchestrator_core::{provisioner::Type, start};

#[tokio::main]
async fn main() {
    let provisioner = DockerProvisioner::new();
    start(Type::Docker, provisioner).await;
}
