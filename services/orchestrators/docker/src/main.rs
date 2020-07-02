use std::env;

mod provisioner;

use crate::provisioner::DockerProvisioner;
use orchestrator_core::{provisioner::parse_images_string, provisioner::Type, start};

#[tokio::main]
async fn main() {
    let images_string = env::var("WEBGRID_IMAGES").unwrap_or_default();
    let images = parse_images_string(images_string);

    let provisioner = DockerProvisioner::new(images);
    start(Type::Docker, provisioner).await;
}
