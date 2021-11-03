//! Various small helper functions
use hyper::body;
use hyper::{http::Uri, Client};
use std::collections::HashSet;
use std::convert::Infallible;
use std::io::Read;
use std::num::ParseIntError;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, instrument, trace};

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
#[instrument]
pub fn load_config(name: &str) -> Result<String, std::io::Error> {
    let directory = std::env::var("WEBGRID_CONFIG_DIR").unwrap_or_else(|_| "/configs".to_string());
    let path = std::path::Path::new(&directory).join(name);

    debug!(?path, "Loading config");

    let mut file = std::fs::File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    debug!(bytes = data.len(), "Loaded config");

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

/// Parses a string containing a list of strings separated by commas
/// Useful for command line parsing
pub fn parse_string_list(src: &str) -> Result<HashSet<String>, Infallible> {
    Ok(src
        .split(',')
        .map(&str::to_owned)
        .collect::<HashSet<String>>())
}

/// Sends HTTP requests to the specified URL until either a 200 OK response is received or the timeout is reached
#[instrument]
pub async fn wait_for(url: &str, timeout_duration: Duration) -> Result<String, ()> {
    let client = Client::new();

    let url = url.parse::<Uri>().unwrap();

    let check_interval = Duration::from_millis(250);
    let request_timeout = Duration::from_millis(1000);
    let mut remaining_duration = timeout_duration;

    debug!("Awaiting OK response");

    loop {
        let mut req = hyper::Request::new(hyper::Body::default());
        *req.uri_mut() = url.clone();

        trace!("Sending request");
        let request = client.request(req);
        let response = timeout(request_timeout, request).await;

        match response {
            Ok(Ok(res)) => {
                trace!(status = ?res.status(), "Received response");

                if res.status() == 200 {
                    debug!("Endpoint became healthy");
                    return match body::to_bytes(res.into_body()).await {
                        Ok(bytes) => Ok(
                            String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string())
                        ),
                        Err(_) => Ok("".to_string()),
                    };
                }
            }
            Ok(Err(error)) => trace!(?error, "Request failed"),
            Err(_) => trace!("Request timed out"),
        }

        if remaining_duration.as_secs() == 0 {
            debug!("Wait timeout reached");
            return Err(());
        }

        trace!(delay = check_interval.as_secs(), "Delaying next request");
        sleep(check_interval).await;
        remaining_duration -= check_interval;
    }
}

/// Wrapper around [`chrono_datetime_as_bson_datetime`](mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime) that supports `Option<DateTime>`
pub mod option_chrono_datetime_as_bson_datetime {
    use std::fmt;

    use chrono::{DateTime, Utc};
    use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
    use serde::{
        de::{Error, Visitor},
        Deserializer, Serializer,
    };

    struct OptionalDateTimeFromCustomFormatVisitor;

    impl<'de> Visitor<'de> for OptionalDateTimeFromCustomFormatVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "null or a datetime string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, d: D) -> Result<Option<DateTime<Utc>>, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(Some(chrono_datetime_as_bson_datetime::deserialize(d)?))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }
    }

    /// Serializes from [`chrono`] to [`bson`](mongodb::bson) implementation of [`DateTime`]
    pub fn serialize<S: Serializer>(
        val: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match val {
            Some(date) => chrono_datetime_as_bson_datetime::serialize(date, serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Deserializes from [`bson`](mongodb::bson) to [`chrono`] implementations of [`DateTime`]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionalDateTimeFromCustomFormatVisitor)
    }
}
