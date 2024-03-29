use std::time::Duration;

use crate::options::{QueueingOptions, RedisOptions};
use domain::container::ContainerImageSet;
use library::helpers::parse_seconds;
use structopt::StructOpt;

/// Options for the orchestrator module and provisioner
#[derive(Debug, StructOpt)]
pub struct Options {
    #[allow(missing_docs)]
    #[structopt(subcommand)]
    pub provisioner: ProvisionerCommand,
}

/// Options for the orchestrator module
#[derive(Debug, StructOpt)]
pub struct OrchestratorOptions {
    /// Maximum number of sessions managed by this instance.
    /// When this number is reached, provisioning requests have to wait
    /// until a running session terminates or use another orchestrator.
    #[structopt(short, long, env)]
    pub permits: usize,

    #[structopt(long, env, default_value = "30", parse(try_from_str = parse_seconds))]
    pub cleanup_interval: Duration,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub queueing: QueueingOptions,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub redis: RedisOptions,
}

/// Variants of provisioners
#[derive(Debug, StructOpt)]
pub enum ProvisionerCommand {
    /// Utilizes K8s Jobs to provision browsers
    Kubernetes(KubernetesOptions),
    /// Creates browsers by dispatching docker containers
    Docker(DockerOptions),
}

#[derive(Debug, StructOpt)]
pub struct DockerOptions {
    #[structopt(flatten)]
    pub orchestrator: OrchestratorOptions,

    /// List of images with associated browser versions that should be used.
    /// For more details, please consult the WebGrid documentation regarding
    /// the ContainerImageSet data structure.
    #[structopt(env)]
    pub images: ContainerImageSet,

    /// When this flag is set, all session containers will be kept after they finished.
    /// Note that this may yield a vast amount of exited container so only use sparingly
    /// and primarily for debugging purposes!
    #[structopt(long)]
    pub retain_exited_sessions: bool,

    /// Storage URL which will be passed on to newly created sessions
    #[structopt(env, long)]
    pub storage: Option<String>,

    /// Log level for new sessions, scopable to different modules
    ///
    /// Levels: trace, debug, info, warn, error
    #[structopt(
        name = "session-log",
        long,
        default_value = "info,hyper=warn,warp=warn,sqlx=warn,tower=warn,h2=warn",
        env = "SESSION_LOG",
        value_name = "sessionLevel"
    )]
    pub log: String,

    /// Volume binds for session containers using the docker format (e.g. /tmp/onHost:/insideContainer).
    /// Using this option is not recommended and it will be deprecated at a later point.
    #[structopt(env, long)]
    pub volume: Vec<String>,
}

#[derive(Debug, StructOpt)]
pub struct KubernetesOptions {
    #[structopt(flatten)]
    pub orchestrator: OrchestratorOptions,

    /// List of images with associated browser versions that should be used.
    /// For more details, please consult the WebGrid documentation regarding
    /// the ContainerImageSet data structure.
    #[structopt(env)]
    pub images: ContainerImageSet,
}
