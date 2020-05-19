use redis::aio::ConnectionManager;
use shared::database::connect;
use shared::lifecycle::Heart;
use shared::logging::Logger;
use shared::metrics::{MetricsEntry, MetricsProcessor};
use tokio::sync::mpsc::UnboundedSender;

use crate::config::Config;

pub struct Context {
    pub config: Config,
    pub con: ConnectionManager,
    pub logger: Logger,
    pub heart: Heart,
    pub metrics_tx: UnboundedSender<MetricsEntry>,
}

impl Context {
    pub async fn new() -> (Self, MetricsProcessor) {
        let config = Config::new().unwrap();
        let con = connect(config.clone().redis_url).await;

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

    pub async fn create_client(&self) -> ConnectionManager {
        connect(self.config.clone().redis_url).await
    }
}
