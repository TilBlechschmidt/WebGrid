use anyhow::Result;
use log::info;
use options::Command;
use structopt::StructOpt;
use webgrid::harness::ModuleRunner;
use webgrid::module::gangway::Gangway;
use webgrid::module::manager::Manager;
use webgrid::module::node::Node;
use webgrid::module::orchestrator::Orchestrator;

mod options;
mod telemetry;

#[tokio::main]
async fn main() -> Result<()> {
    let (command, runner) = init().await?;

    match command {
        Command::Node(options) => runner.run(Node::new(options)).await,
        Command::Manager(options) => runner.run(Manager::new(options)).await,
        Command::Orchestrator(options) => runner.run(Orchestrator::new(options)).await,
        Command::Gangway(options) => runner.run(Gangway::new(options)).await,
    };

    deinit();

    Ok(())
}

async fn init() -> Result<(options::Command, ModuleRunner)> {
    let options = options::MainOptions::from_args();

    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&options.log)
        .try_init()?;

    if let Some(telemetry_endpoint) = options.telemetry_endpoint {
        telemetry::try_init(&telemetry_endpoint)?;
    }

    let runner = match options.status_server {
        Some(port) => ModuleRunner::new_with_status_server(port),
        None => ModuleRunner::default(),
    };

    info!("{}", env!("WEBGRID_VERSION"));

    Ok((options.command, runner))
}

fn deinit() {
    telemetry::flush();
}
