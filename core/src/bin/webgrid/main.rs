use anyhow::Result;
use options::{Command, LogFormat};
use structopt::StructOpt;
use tracing::info;
use webgrid::harness::ModuleRunner;
use webgrid::module::api::Api;
use webgrid::module::collector::Collector;
use webgrid::module::gangway::Gangway;
use webgrid::module::manager::Manager;
use webgrid::module::node::Node;
use webgrid::module::orchestrator::Orchestrator;

mod options;

#[tokio::main]
async fn main() -> Result<()> {
    let (command, runner) = init().await?;

    match command {
        Command::Node(options) => runner.run(Node::new(options)).await,
        Command::Manager(options) => runner.run(Manager::new(options)).await,
        Command::Orchestrator(options) => runner.run(Orchestrator::new(options)).await,
        Command::Gangway(options) => runner.run(Gangway::new(options)).await,
        Command::Collector(options) => runner.run(Collector::new(options)).await,
        Command::Api(options) => runner.run(Api::new(options)).await,
    };

    deinit();

    Ok(())
}

async fn init() -> Result<(options::Command, ModuleRunner)> {
    let options = options::MainOptions::from_args();

    let formatter = tracing_subscriber::fmt().with_env_filter(options.log);

    match options.log_format {
        LogFormat::Text => formatter.init(),
        LogFormat::Compact => formatter.compact().init(),
        LogFormat::Json => formatter.json().init(),
    };

    let runner = match options.status_server {
        Some(port) => ModuleRunner::new_with_status_server(port),
        None => ModuleRunner::default(),
    };

    info!("WebGrid {}", env!("WEBGRID_VERSION"));

    Ok((options.command, runner))
}

fn deinit() {}
