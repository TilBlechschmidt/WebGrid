use structopt::{clap::arg_enum, StructOpt};

arg_enum! {
    #[derive(Debug)]
    pub enum LogFormat {
        Text,
        Compact,
        Json
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Decentralized, scalable and robust selenium-grid equivalent.",
    version = env!("WEBGRID_VERSION")
)]
pub struct MainOptions {
    /// Log level, scopable to different modules
    ///
    /// Levels: trace, debug, info, warn, error
    #[structopt(
        short,
        long,
        global = true,
        default_value = "info,hyper=warn,warp=warn,sqlx=warn,tower=warn,h2=warn",
        env = "RUST_LOG",
        value_name = "level"
    )]
    pub log: String,

    /// Formatting style for log outputs
    #[structopt(long, global = true, env, possible_values = &LogFormat::variants(), case_insensitive = true, default_value = "Compact")]
    pub log_format: LogFormat,

    /// OpenTelemetry collector endpoint
    ///
    /// Omitting it disables tracing
    #[structopt(long, global = true, env)]
    pub telemetry_endpoint: Option<String>,

    /// Enable status reporting server which can be used as a readiness probe
    #[structopt(long, global = true, env, value_name = "port")]
    pub status_server: Option<u16>,

    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    Node(webgrid::module::node::Options),
    Manager(webgrid::module::manager::Options),
    Orchestrator(webgrid::module::orchestrator::Options),
    Gangway(webgrid::module::gangway::Options),
    Collector(webgrid::module::collector::Options),
    Api(webgrid::module::api::Options),
}
