use super::super::super::SharedOptions;
use crate::libraries::helpers::constants;
use structopt::StructOpt;

mod provisioner;

use super::super::core::{
    provisioner::parse_images_string, provisioner::Type, start, Options as CoreOptions,
};
use provisioner::K8sProvisioner;

#[derive(Debug, StructOpt)]
/// Kubernetes job based session provisioner
pub struct Options {
    /// Port on which nodes will be reachable
    #[structopt(short, long, default_value = constants::PORT_NODE)]
    node_port: u16,

    /// List of images and associated browser versions
    ///
    /// Example: "webgrid-node-firefox=firefox::68.7.0esr,webgrid-node-chrome=chrome::81.0.4044.122"
    #[structopt(long, env)]
    images: String,
}

pub async fn run(shared_options: SharedOptions, core_options: CoreOptions, options: Options) {
    let images = parse_images_string(options.images);

    let provisioner = K8sProvisioner::new(options.node_port, images).await;
    start(Type::K8s, provisioner, core_options, shared_options).await;
}
