extern crate pretty_env_logger;

use log::{info, trace};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use std::fmt;

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub mod capabilities;
pub mod lifecycle;
pub mod logging;
pub mod metrics;

pub fn service_init() {
    pretty_env_logger::init_timed();
}

// Various timeouts in seconds
#[derive(Debug)]
pub enum Timeout {
    Queue,
    Scheduling,
    NodeStartup,
    DriverStartup,
    SessionTermination,
    SlotReclaimInterval,
}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Timeout {
    fn default(&self) -> usize {
        match *self {
            // Manager
            Timeout::Queue => 120,
            Timeout::Scheduling => 60,
            Timeout::NodeStartup => 45,
            // Node
            Timeout::DriverStartup => 30,
            Timeout::SessionTermination => 60,
            // Orchestrator
            Timeout::SlotReclaimInterval => 300,
        }
    }

    pub async fn get(&self, con: &MultiplexedConnection) -> usize {
        let mut con = con.clone();
        let key = format!("{}", self).to_lowercase();

        trace!("Reading timeout {}", key);
        let timeout: Option<usize> = con.hget("timeouts", &key).await.ok();

        match timeout {
            Some(timeout) => timeout,
            None => {
                info!("Initializing timeout {} to default value", key);
                let default = self.default();
                let _: Option<()> = con.hset("timeouts", key, default).await.ok();
                default
            }
        }
    }
}

pub fn load_config(name: &str) -> String {
    let directory = std::env::var("WEBGRID_CONFIG_DIR").unwrap_or_else(|_| "/configs".to_string());
    let path = Path::new(&directory).join(name);
    let mut file = File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    data
}

pub fn replace_config_variable(config: String, key: &str, value: &str) -> String {
    config.replace(&format!("{{{{{}}}}}", key), &value.to_string())
}

pub fn split_into_two(input: &str, separator: &'static str) -> Option<(String, String)> {
    let parts: Vec<&str> = input.splitn(2, separator).collect();

    if parts.len() != 2 {
        return None;
    }

    Some((parts[0].to_string(), parts[1].to_string()))
}

pub fn parse_browser_string(input: &str) -> Option<(String, String)> {
    split_into_two(input, "::")
}
