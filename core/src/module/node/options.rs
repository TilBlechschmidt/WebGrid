use crate::domain::webdriver::{ScreenResolution, WebDriverVariant};
use crate::library::helpers::parse_seconds;
use crate::module::options::RedisOptions;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;
use uuid::Uuid;

/// Options for the manager module
#[derive(Debug, StructOpt)]
pub struct Options {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub redis: RedisOptions,

    /// Unique identifier of the linked session object
    #[structopt(env)]
    pub id: Uuid,

    /// Idle timeout (in seconds) which is in effect until the first client request is received.
    /// This allows the session to terminate early if the client no longer has any interest
    /// in the session or it itself ran into a local timeout (e.g. due to prolonged queueing).
    /// After the first request from a client has been received, the regular idle-timeout is
    /// taking effect.
    #[structopt(long, env, default_value = "30", parse(try_from_str = parse_seconds))]
    pub initial_timeout: Duration,

    /// If no WebDriver client request is received within the specified period, the node will
    /// terminate. Each incoming request resets the countdown.
    #[structopt(long, env, default_value = "120", parse(try_from_str = parse_seconds))]
    pub idle_timeout: Duration,

    /// Options relating to the WebDriver
    #[structopt(flatten)]
    pub webdriver: WebDriverOptions,

    /// Hostname or IP address where this instance can be reached by proxy services
    #[structopt(short, long, env)]
    pub host: String,
}

/// WebDriver related options
#[derive(Debug, StructOpt)]
pub struct WebDriverOptions {
    /// Location of the WebDriver executable
    #[structopt(env = "DRIVER")]
    pub binary: PathBuf,

    /// Variant of the WebDriver
    #[structopt(long, env = "DRIVER_VARIANT")]
    pub variant: WebDriverVariant,

    /// Screen resolution for new sessions
    #[structopt(long, env, default_value = "1920x1080")]
    pub resolution: ScreenResolution,

    /// Maximum duration (in seconds) the webdriver may take until it reports a ready state
    #[structopt(long, env, default_value = "15", parse(try_from_str = parse_seconds))]
    pub startup_timeout: Duration,

    /// Capabilities object which will be used to create a session with the driver (formatted as JSON)
    #[structopt(env)]
    pub capabilities: String,
}
