mod provisioner;

use crate::provisioner::K8sProvisioner;
use orchestrator_core::{provisioner::parse_images_string, provisioner::Type, start};
use std::env;

#[tokio::main]
async fn main() {
    let images_string = env::var("WEBGRID_IMAGES").unwrap_or_default();
    let images = parse_images_string(images_string);

    let provisioner = K8sProvisioner::new(images).await;
    start(Type::K8s, provisioner).await;
}
