use redis::{aio::MultiplexedConnection, Client};
use shared::lifecycle::Heart;
use shared::logging::Logger;
use shared::metrics::{MetricsEntry, MetricsProcessor};
use tokio::sync::mpsc::UnboundedSender;

use crate::config::Config;

pub struct Context {
    pub config: Config,
    pub con: MultiplexedConnection,
    pub logger: Logger,
    pub heart: Heart,
    pub metrics_tx: UnboundedSender<MetricsEntry>,
}

impl Context {
    pub async fn new() -> (Self, MetricsProcessor) {
        let config = Config::new().unwrap();

        let client = Client::open(config.clone().redis_url).unwrap();
        let con = client.get_multiplexed_tokio_connection().await.unwrap();

        let logger = Logger::new(&con, "manager".to_string());
        let heart = Heart::new(&con, None);

        let metrics = MetricsProcessor::new(&con);

        let ctx = Self {
            config,
            con,
            logger,
            heart,
            metrics_tx: metrics.get_tx(),
        };

        (ctx, metrics)
    }
}
