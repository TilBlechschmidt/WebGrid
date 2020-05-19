use redis::aio::ConnectionManager;
use shared::database::connect;
use shared::lifecycle::Heart;
use shared::logging::SessionLogger;
use shared::Timeout;
use std::net::SocketAddr;

use crate::config::Config;
use crate::driver::DriverManager;

pub struct Context {
    pub config: Config,
    pub con: ConnectionManager,
    pub logger: SessionLogger,
    pub driver: DriverManager,
    pub driver_addr: SocketAddr,
    pub heart: Heart,
}

impl Context {
    pub async fn new() -> Self {
        let config = Config::new().unwrap();

        let con = connect(config.clone().redis_url).await;

        let logger = SessionLogger::new(&con, "node".to_string(), config.session_id.clone());
        let heart = Heart::new(&con, Some(Timeout::SessionTermination.get(&con).await));

        Context {
            driver: DriverManager::new(config.driver.clone()),
            driver_addr: ([127, 0, 0, 1], config.driver_port).into(),
            config,
            con,
            logger,
            heart,
        }
    }

    pub fn get_driver_url(&self, path: &str) -> String {
        format!("http://{}{}", self.driver_addr.to_string(), path)
    }
}
