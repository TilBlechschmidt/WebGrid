use crate::options::{QueueingOptions, RedisOptions};
use library::helpers::parse_string_list;
use std::collections::HashSet;
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

    /// Metadata keys which clients are required to provide, separated by commas.
    /// Omitting this flag or setting an empty string will allow requests without metadata.
    #[structopt(long, env, default_value = "", parse(try_from_str = parse_string_list))]
    pub required_metadata: HashSet<String>,
}
