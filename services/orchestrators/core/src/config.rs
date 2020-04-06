use config::{Config as ConfigLoader, ConfigError, Environment};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub orchestrator_id: String,
    pub slots: usize,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = ConfigLoader::new();

        s.set_default("redis_url", "redis://webgrid-redis/")?;
        s.merge(Environment::with_prefix("WEBGRID"))?;

        s.try_into()
    }
}
