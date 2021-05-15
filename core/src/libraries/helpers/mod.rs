//! Helper functions that don't belong elsewhere
//!
//! This module contains small helper functions that don't belong to a specific area but are still used by multiple services.

mod capabilities;
mod healthcheck;

pub mod constants;
pub mod keys;
pub mod lua;

use std::{num::ParseIntError, time::Duration};

pub use capabilities::*;
pub use healthcheck::{wait_for, wait_for_key};

/// Splits the input string into two parts at the first occurence of the separator
pub fn split_into_two(input: &str, separator: &'static str) -> Option<(String, String)> {
    let parts: Vec<&str> = input.splitn(2, separator).collect();

    if parts.len() != 2 {
        return None;
    }

    Some((parts[0].to_string(), parts[1].to_string()))
}

/// Parses a browser string into a name and version
pub fn parse_browser_string(input: &str) -> Option<(String, String)> {
    split_into_two(input, "::")
}

/// Reads a config file by name from the default config directory or one that is specified by the `WEBGRID_CONFIG_DIR` env variable.
pub fn load_config(name: &str) -> String {
    use std::io::Read;

    let directory = std::env::var("WEBGRID_CONFIG_DIR").unwrap_or_else(|_| "/configs".to_string());
    let path = std::path::Path::new(&directory).join(name);
    let mut file = std::fs::File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    data
}

/// Replaces a variable in the passed config string
pub fn replace_config_variable(config: String, key: &str, value: &str) -> String {
    config.replace(&format!("{{{{{}}}}}", key), &value.to_string())
}

/// Parses a Duration from a string containing seconds.
/// Useful for command line parsing
pub fn parse_seconds(src: &str) -> Result<Duration, ParseIntError> {
    let seconds = src.parse::<u64>()?;
    Ok(Duration::from_secs(seconds))
}
