use crate::options::{MongoDBOptions, QueueingOptions, RedisOptions};
use structopt::StructOpt;

/// Options for the manager module
#[derive(Debug, StructOpt)]
pub struct Options {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub queueing: QueueingOptions,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub redis: RedisOptions,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub mongo: MongoDBOptions,
}
