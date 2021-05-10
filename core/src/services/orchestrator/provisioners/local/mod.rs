use super::super::super::SharedOptions;
use anyhow::Result;
use structopt::StructOpt;

mod provisioner;

use super::super::core::{provisioner::Type, start, Options as CoreOptions};
use provisioner::LocalProvisioner;

#[derive(Debug, StructOpt)]
/// Local process based session provisioner
pub struct Options {}

pub async fn run(
    shared_options: SharedOptions,
    core_options: CoreOptions,
    _options: Options,
) -> Result<()> {
    assert_eq!(
        core_options.slot_count, 1,
        "Local orchestrator only allows a single slot."
    );

    let provisioner = LocalProvisioner::new(
        shared_options.redis.clone(),
        shared_options.log.clone(),
        shared_options.trace_endpoint.clone(),
    );
    start(Type::Local, provisioner, core_options, shared_options).await?;

    Ok(())
}
