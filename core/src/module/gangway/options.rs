use crate::module::options::{QueueingOptions, RedisOptions};
use structopt::StructOpt;

/// Options for the gangway module
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
    pub service_discovery: ServiceDiscoveryOptions,

    /// Maximum number of concurrent, pending session creation requests.
    /// When more requests arrive, the oldest ones will be terminated. In reality,
    /// this variable is only here to cap the memory usage and not to actively control the requests.
    /// When you are hitting this limit you should probably start scaling horizontally instead.
    #[structopt(long, env, default_value = "25000")]
    pub pending_request_limit: usize,
}

#[derive(Debug, StructOpt)]
pub struct ServiceDiscoveryOptions {
    /// Maximum number of cached service endpoints
    #[structopt(long, env, default_value = "1000")]
    pub cache_size: usize,
    /// Allowed number of pending service discovery requests.
    /// If for some reason the requests processing is slow, at most this
    /// number of pending requests will be kept in a queue. When exceeded,
    /// the oldest requests will be dropped.
    #[structopt(long, env, default_value = "10000")]
    pub request_channel_size: usize,
    /// Allowed number of pending service discovery responses.
    /// If one of the worker threads is running behind in processing discovery
    /// responses, only this amount will be retained. The oldest unprocessed entries
    /// will be dropped (similar to request_channel_size but for responses).
    #[structopt(long, env, default_value = "10000")]
    pub response_channel_size: usize,
}
