use redis::{aio::MultiplexedConnection, Client};
use shared::lifecycle::Heart;
use shared::logging::SessionLogger;
use shared::Timeout;

use crate::config::Config;
use crate::driver::DriverManager;

pub struct Context {
    pub config: Config,
    pub con: MultiplexedConnection,
    pub logger: SessionLogger,
    pub driver: DriverManager,
    pub heart: Heart,
}

impl Context {
    pub async fn new() -> Self {
        let config = Config::new().unwrap();

        let client = Client::open(config.clone().redis_url).unwrap();
        let con = client.get_multiplexed_tokio_connection().await.unwrap();

        let logger = SessionLogger::new(&con, "node".to_string(), config.session_id.clone());
        let heart = Heart::new(&con, Some(Timeout::SessionTermination.get(&con).await));

        Context {
            driver: DriverManager::new(config.driver.clone()),
            config,
            con,
            logger,
            heart,
        }
    }
}
