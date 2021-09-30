use crate::module::options::{MongoDBOptions, RedisOptions};
use std::path::PathBuf;
use structopt::StructOpt;

/// Options for the API module
#[derive(Debug, StructOpt)]
pub struct Options {
    /// Directory from which files will be served
    #[structopt(long, env, parse(from_os_str), default_value = "/www")]
    pub web_root: PathBuf,

    /// Hostname or IP address where this instance can be reached by proxy services
    #[structopt(short, long, env)]
    pub host: String,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub redis: RedisOptions,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub mongo: MongoDBOptions,
}
