mod provisioner;

use crate::provisioner::DockerProvisioner;
use orchestrator_core::start;

#[tokio::main]
async fn main() {
    let provisioner = DockerProvisioner::new();
    start(provisioner).await;
}
