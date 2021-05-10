use super::super::super::SharedOptions;
use crate::libraries::helpers::constants;
use anyhow::Result;
use structopt::StructOpt;

mod provisioner;

use super::super::core::{
    provisioner::parse_images_string, provisioner::Type, start, Options as CoreOptions,
};
use provisioner::DockerProvisioner;

#[derive(Debug, StructOpt)]
/// Docker container based session provisioner
pub struct Options {
    /// Port on which nodes will be reachable
    #[structopt(short, long, default_value = constants::PORT_NODE)]
    node_port: u16,

    /// List of docker images and associated browser versions
    ///
    /// Example: "webgrid-node-firefox=firefox::68.7.0esr,webgrid-node-chrome=chrome::81.0.4044.122"
    #[structopt(long, env)]
    images: String,

    /// Do not enable video recordings for spawned sessions
    #[structopt(long, env)]
    disable_recording: bool,
}

pub async fn run(
    shared_options: SharedOptions,
    core_options: CoreOptions,
    options: Options,
) -> Result<()> {
    let images = parse_images_string(options.images);

    let provisioner = DockerProvisioner::new(
        options.node_port,
        images,
        options.disable_recording,
    )?;

    start(Type::Docker, provisioner, core_options, shared_options).await?;

    Ok(())
}
