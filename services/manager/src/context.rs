use redis::{aio::MultiplexedConnection, Client};
use shared::lifecycle::Heart;
use shared::logging::Logger;

use crate::config::Config;

pub struct Context {
    pub config: Config,
    pub con: MultiplexedConnection,
    pub logger: Logger,
    pub heart: Heart,
}

impl Context {
    pub async fn new() -> Self {
        let config = Config::new().unwrap();

        let client = Client::open(config.clone().redis_url).unwrap();
        let con = client.get_multiplexed_tokio_connection().await.unwrap();

        let logger = Logger::new(&con, "manager".to_string());
        let heart = Heart::new(&con, None);

        Context {
            config,
            con,
            logger,
            heart,
        }
    }
}
