//! HTTP healthcheck related functions
//!
//! Functions that are used to check if a HTTP endpoint is reachable

use hyper::{body, Client, Uri};
use log::{debug, trace};
use std::time::Duration;
use tokio::time::delay_for;
use tokio::time::timeout;

/// Sends HTTP requests to the specified URL until either a 200 OK response is received or the timeout is reached
pub async fn wait_for(url: &str, timeout_duration: Duration) -> Result<String, ()> {
    let client = Client::new();

    let url = url.parse::<Uri>().unwrap();

    let check_interval = Duration::from_millis(250);
    let request_timeout = Duration::from_millis(1000);
    let mut remaining_duration = timeout_duration;

    debug!("Awaiting 200 OK response from {}", url);

    loop {
        let request = client.get(url.clone());

        trace!("Sending health-check request");
        let response = timeout(request_timeout, request).await;

        // Rust does not yet support boolean and let in the same IF statement. TODO Replace this once language support lands
        if let Ok(Ok(res)) = response {
            if res.status() == 200 {
                return match body::to_bytes(res.into_body()).await {
                    Ok(bytes) => {
                        Ok(String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()))
                    }
                    Err(_) => Ok("".to_string()),
                };
            }

            trace!("Received response with status != 200");
        } else {
            trace!("Unable to send request to node! {:?}", response);
        }

        if remaining_duration.as_secs() == 0 {
            debug!("Timeout while waiting for {}", url);
            return Err(());
        }

        delay_for(check_interval).await;
        remaining_duration -= check_interval;
    }
}
