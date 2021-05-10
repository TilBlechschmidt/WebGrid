use anyhow::Result;
use opentelemetry::global;
use structopt::StructOpt;

#[cfg(feature = "orchestrator")]
use webgrid::services::orchestrator::{provisioners, Provisioner};
use webgrid::services::*;

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Decentralized, scalable and robust selenium-grid equivalent.",
    version = env!("WEBGRID_VERSION")
)]
struct MainOptions {
    #[structopt(flatten)]
    shared_options: SharedOptions,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[cfg(feature = "metrics")]
    Metrics(metrics::Options),

    #[cfg(feature = "manager")]
    Manager(manager::Options),

    #[cfg(feature = "node")]
    Node(node::Options),

    #[cfg(feature = "proxy")]
    Proxy(proxy::Options),

    #[cfg(feature = "storage")]
    Storage(storage::Options),

    #[cfg(feature = "orchestrator")]
    Orchestrator(orchestrator::Options),

    #[cfg(feature = "gc")]
    Gc(gc::Options),

    #[cfg(feature = "api")]
    Api(api::Options),
}

#[tokio::main]
async fn main() -> Result<()> {
    let main_options = MainOptions::from_args();
    let shared_options = main_options.shared_options;

    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&shared_options.log)
        .init();

    log::info!("{}", env!("WEBGRID_VERSION"));

    match main_options.cmd {
        #[cfg(feature = "metrics")]
        Command::Metrics(options) => metrics::run(shared_options, options).await,

        #[cfg(feature = "manager")]
        Command::Manager(options) => manager::run(shared_options, options).await?,

        #[cfg(feature = "node")]
        Command::Node(options) => node::run(shared_options, options).await?,

        #[cfg(feature = "proxy")]
        Command::Proxy(options) => proxy::run(shared_options, options).await?,

        #[cfg(feature = "storage")]
        Command::Storage(options) => storage::run(shared_options, options).await?,

        #[cfg(feature = "orchestrator")]
        Command::Orchestrator(core_options) => match core_options.provisioner {
            #[cfg(feature = "docker")]
            Provisioner::Docker(options) => {
                provisioners::docker::run(shared_options, core_options.core, options).await?
            }

            #[cfg(feature = "kubernetes")]
            Provisioner::Kubernetes(options) => {
                provisioners::kubernetes::run(shared_options, core_options.core, options).await?
            }
        },

        #[cfg(feature = "gc")]
        Command::Gc(options) => gc::run(shared_options, options).await?,

        #[cfg(feature = "api")]
        Command::Api(options) => api::run(shared_options, options).await?,
    }

    global::shutdown_tracer_provider();

    Ok(())
}
