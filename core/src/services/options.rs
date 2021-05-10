use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct SharedOptions {
    /// Redis database server URL
    #[structopt(
        short,
        long,
        global = true,
        env,
        default_value = "redis://webgrid-redis/",
        value_name = "url"
    )]
    pub redis: String,

    /// Enable status reporting server with optional port.
    ///
    /// If the flag is used without a port it will default to 47002.
    #[structopt(long, global = true, env, value_name = "port")]
    pub status_server: Option<Option<u16>>,

    /// Log level, scopable to different modules
    ///
    /// Levels: trace, debug, info, warn, error
    #[structopt(
        short,
        long,
        global = true,
        default_value = "warn",
        env = "RUST_LOG",
        value_name = "level"
    )]
    pub log: String,

    /// OpenTelemetry collector endpoint
    ///
    /// Omitting it disables tracing
    #[structopt(long, global = true, env)]
    pub trace_endpoint: Option<String>,
}
