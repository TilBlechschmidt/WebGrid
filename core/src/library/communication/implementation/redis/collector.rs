use super::super::super::super::BoxedError;
use super::super::super::request::{
    RawResponseCollector, ResponseCollectionTimeout, ResponseLocation,
};
use super::super::json::JsonResponseCollector;
use super::RedisConnectionVariant;
use super::RedisFactory;
use super::RESPONSE_KEY_PREFIX;
use async_trait::async_trait;
use futures::stream;
use futures::stream::BoxStream;
use futures::StreamExt;
use redis::AsyncCommands;
use std::convert::TryInto;
use std::time::Duration;
use std::time::Instant;

/// Response collector implementation using [Redis Lists](https://redis.io/topics/data-types#lists)
pub struct RedisResponseCollector<F: RedisFactory> {
    connection_factory: F,
}

impl<F: RedisFactory> RedisResponseCollector<F> {
    /// Creates a new instance from a given [`RedisFactory`]
    pub fn new(connection_factory: F) -> Self {
        Self { connection_factory }
    }
}

#[async_trait]
impl<F> RawResponseCollector for RedisResponseCollector<F>
where
    F: RedisFactory + Send + Sync,
{
    async fn collect_raw(
        &self,
        location: ResponseLocation,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<BoxStream<Result<Vec<u8>, BoxedError>>, BoxedError> {
        let connection = self
            .connection_factory
            .connection(RedisConnectionVariant::Pooled)
            .await?;
        let timeout_tracker = TimeoutTracker::new(timeout);

        let stream = stream::unfold(
            (connection, timeout_tracker, limit),
            move |(mut con, mut timeout_tracker, mut remaining_limit)| {
                let key = format!("{}{}", RESPONSE_KEY_PREFIX, location);
                async move {
                    if remaining_limit == Some(0) {
                        None
                    } else if let Some(remaining_timeout) = timeout_tracker.current_timeout_secs() {
                        let remaining_timeout: usize =
                            remaining_timeout.try_into().unwrap_or_default();

                        let result = con
                            .blpop::<_, Option<(String, Vec<u8>)>>(&key, remaining_timeout)
                            .await
                            .map(|o| o.map(|i| i.1))
                            .map_err(Into::into);

                        timeout_tracker.processed_item();

                        if let Some(limit) = remaining_limit {
                            remaining_limit = Some(limit - 1);
                        }

                        // Handle the timeout expiring while we are blocking (aka Ok(None))
                        match result {
                            Ok(Some(response)) => {
                                Some((Ok(response), (con, timeout_tracker, remaining_limit)))
                            }
                            Err(e) => Some((Err(e), (con, timeout_tracker, remaining_limit))),
                            /* Ok(None) */ _ => None,
                        }
                    } else {
                        None
                    }
                }
            },
        );

        Ok(stream.boxed())
    }
}

impl<F> JsonResponseCollector for RedisResponseCollector<F> where F: RedisFactory + Send + Sync {}

/// Helper struct to calculate the next timeout that should be passed to `BLPOP`
struct TimeoutTracker {
    start: Instant,
    mode: ResponseCollectionTimeout,
}

impl TimeoutTracker {
    fn new(mode: ResponseCollectionTimeout) -> Self {
        Self {
            start: Instant::now(),
            mode,
        }
    }

    fn current_timeout_secs(&self) -> Option<u64> {
        if let Some(duration) = self.current_timeout() {
            if duration.as_secs() < 1 {
                None
            } else {
                Some(duration.as_secs())
            }
        } else {
            Some(0)
        }
    }

    fn current_timeout(&self) -> Option<Duration> {
        match self.mode {
            ResponseCollectionTimeout::None => None,
            ResponseCollectionTimeout::TotalDuration(duration) => {
                Some(duration - self.start.elapsed())
            }
            ResponseCollectionTimeout::Split(initial_duration, _) => {
                Some(initial_duration - self.start.elapsed())
            }
        }
    }

    fn processed_item(&mut self) {
        match self.mode {
            ResponseCollectionTimeout::None => {}
            ResponseCollectionTimeout::TotalDuration(_) => {}
            ResponseCollectionTimeout::Split(_, timeout) => {
                self.start = Instant::now();
                // We add an extra second because current_timeout_secs will "round down" the number of seconds.
                // As function calls are not happening simultaneously, after this function finishes, current_timeout_secs
                // would return one less second than you would expect (because it actually reads something like `timeout - 1ns`).
                self.mode =
                    ResponseCollectionTimeout::TotalDuration(timeout + Duration::from_secs(1));
            }
        }
    }
}
