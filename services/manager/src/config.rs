use config::{Config as ConfigLoader, ConfigError, Environment};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub manager_id: String,
    pub manager_host: String,
    pub manager_port: u64,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = ConfigLoader::new();

        s.set_default("redis_url", "redis://webgrid-redis/")?;
        s.set_default("manager_port", 3033)?;

        s.merge(Environment::with_prefix("WEBGRID"))?;

        s.try_into()
    }
}
