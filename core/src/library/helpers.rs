//! Various small helper functions

use hyper::body;
use hyper::{http::Uri, Client};
use std::num::ParseIntError;
use std::time::Duration;
use tokio::time::{sleep, timeout};

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
pub fn load_config(name: &str) -> Result<String, std::io::Error> {
    use std::io::Read;

    let directory = std::env::var("WEBGRID_CONFIG_DIR").unwrap_or_else(|_| "/configs".to_string());
    let path = std::path::Path::new(&directory).join(name);
    let mut file = std::fs::File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    Ok(data)
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

/// Sends HTTP requests to the specified URL until either a 200 OK response is received or the timeout is reached
pub async fn wait_for(url: &str, timeout_duration: Duration) -> Result<String, ()> {
    let client = Client::new();

    let url = url.parse::<Uri>().unwrap();

    let check_interval = Duration::from_millis(250);
    let request_timeout = Duration::from_millis(1000);
    let mut remaining_duration = timeout_duration;

    log::debug!("Awaiting 200 OK response from {}", url);

    loop {
        let mut req = hyper::Request::new(hyper::Body::default());
        *req.uri_mut() = url.clone();

        let request = client.request(req);

        log::trace!("Sending health-check request");
        let response = timeout(request_timeout, request).await;

        // TODO Replace this once language support lands: Rust does not yet support boolean and let in the same IF statement.
        if let Ok(Ok(res)) = response {
            if res.status() == 200 {
                return match body::to_bytes(res.into_body()).await {
                    Ok(bytes) => {
                        Ok(String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()))
                    }
                    Err(_) => Ok("".to_string()),
                };
            }

            log::trace!("Received response with status != 200");
        } else {
            log::trace!("Unable to send request to node! {:?}", response);
        }

        if remaining_duration.as_secs() == 0 {
            log::debug!("Timeout while waiting for {}", url);
            return Err(());
        }

        sleep(check_interval).await;
        remaining_duration -= check_interval;
    }
}
