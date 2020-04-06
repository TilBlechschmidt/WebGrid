mod provisioner;

use crate::provisioner::K8sProvisioner;
use orchestrator_core::{provisioner::Type, start};

#[tokio::main]
async fn main() {
    let provisioner = K8sProvisioner::new().await;
    start(Type::K8s, provisioner).await;
}
