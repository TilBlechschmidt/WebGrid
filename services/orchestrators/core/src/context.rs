use redis::aio::ConnectionManager;
use shared::database::connect;
use shared::lifecycle::Heart;
use shared::logging::Logger;

use crate::config::Config;

pub struct Context {
    pub config: Config,
    pub con: ConnectionManager,
    pub logger: Logger,
    pub heart: Heart,
}

impl Context {
    pub async fn new() -> Self {
        let config = Config::new().unwrap();

        let con = connect(config.clone().redis_url).await;

        let logger = Logger::new(&con, "orchestrator".to_string());
        let heart = Heart::new(&con, None);

        Context {
            config,
            con,
            logger,
            heart,
        }
    }

    pub async fn create_client(&self) -> ConnectionManager {
        connect(self.config.clone().redis_url).await
    }
}
