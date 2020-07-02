mod backoff;
mod capabilities;
mod healthcheck;
mod ports;
mod timeout;

pub mod env;
pub mod keys;
pub mod lua;

pub use backoff::Backoff;
pub use capabilities::*;
pub use healthcheck::wait_for;
pub use ports::ServicePort;
pub use timeout::Timeout;

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

pub fn load_config(name: &str) -> String {
    use std::io::Read;

    let directory = std::env::var("WEBGRID_CONFIG_DIR").unwrap_or_else(|_| "/configs".to_string());
    let path = std::path::Path::new(&directory).join(name);
    let mut file = std::fs::File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    data
}

pub fn replace_config_variable(config: String, key: &str, value: &str) -> String {
    config.replace(&format!("{{{{{}}}}}", key), &value.to_string())
}
