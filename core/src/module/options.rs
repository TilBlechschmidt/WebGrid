//! Various options usable by modules
//!
//! The structs in this module allow other modules to flatten them into
//! their own options struct. This allows for a unified yet non-cluttered
//! option set.

use structopt::StructOpt;

/// Options for connecting to the Redis server
#[derive(Debug, StructOpt)]
pub struct RedisOptions {
    /// Redis database server URL
    #[structopt(
        short = "r",
        long = "redis",
        env = "REDIS",
        global = true,
        default_value = "redis://webgrid-redis/",
        value_name = "url"
    )]
    pub url: String,
}

/// Options relevant for message queueing
#[derive(Debug, StructOpt)]
pub struct QueueingOptions {
    /// Unique and stable identifier for this instance.
    /// It is used to identify and resume work after a crash
    /// or deliberate restart, thus it may not change across
    /// executions!
    #[structopt(env)]
    pub id: String,
}
