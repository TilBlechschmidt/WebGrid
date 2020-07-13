use anyhow::Result;
use helpers::SharedOptions;
use structopt::StructOpt;

mod orchestrator;

use crate::orchestrator::*;

#[cfg(feature = "service_manager")]
use manager::{run as manager, Options as ManagerOptions};
#[cfg(feature = "service_metrics")]
use metrics::{run as metrics, Options as MetricsOptions};
#[cfg(feature = "service_node")]
use node::{run as node, Options as NodeOptions};
#[cfg(feature = "docker")]
use orchestrator_docker::run as docker;
#[cfg(feature = "kubernetes")]
use orchestrator_k8s::run as k8s;
#[cfg(feature = "proxy")]
use proxy::{run as proxy, Options as ProxyOptions};

#[derive(Debug, StructOpt)]
#[structopt(about = "Decentralized, scalable and robust selenium-grid equivalent.")]
struct MainOptions {
    #[structopt(flatten)]
    shared_options: SharedOptions,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[cfg(feature = "service_metrics")]
    Metrics(MetricsOptions),

    #[cfg(feature = "service_manager")]
    Manager(ManagerOptions),

    #[cfg(feature = "service_node")]
    Node(NodeOptions),

    #[cfg(feature = "service_proxy")]
    Proxy(ProxyOptions),

    #[cfg(feature = "service_orchestrator")]
    Orchestrator(OrchestratorOptions),
}

#[tokio::main]
async fn main() -> Result<()> {
    let main_options = MainOptions::from_args();
    let shared_options = main_options.shared_options;

    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&shared_options.log)
        .init();

    match main_options.cmd {
        #[cfg(feature = "service_metrics")]
        Command::Metrics(options) => metrics(shared_options, options).await,

        #[cfg(feature = "service_manager")]
        Command::Manager(options) => manager(shared_options, options).await,

        #[cfg(feature = "service_node")]
        Command::Node(options) => node(shared_options, options).await?,

        #[cfg(feature = "service_proxy")]
        Command::Proxy(options) => proxy(shared_options, options).await,

        #[cfg(feature = "service_orchestrator")]
        Command::Orchestrator(core_options) => match core_options.provisioner {
            #[cfg(feature = "docker")]
            Provisioner::Docker(options) => {
                docker(shared_options, core_options.core, options).await
            }

            #[cfg(feature = "kubernetes")]
            Provisioner::Kubernetes(options) => {
                k8s(shared_options, core_options.core, options).await
            }
        },
    }

    Ok(())
}
