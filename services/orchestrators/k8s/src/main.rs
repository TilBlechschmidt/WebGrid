mod provisioner;

use crate::provisioner::K8sProvisioner;
use orchestrator_core::start;

#[tokio::main]
async fn main() {
    let provisioner = K8sProvisioner::new();
    start(provisioner).await;
}
