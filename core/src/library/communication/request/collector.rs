use super::super::super::BoxedError;
use super::ResponseLocation;
use async_trait::async_trait;
use futures::stream::BoxStream;

use serde::de::DeserializeOwned;
use std::time::Duration;

/// Timeout structure for response collection
#[derive(PartialEq)]
pub enum ResponseCollectionTimeout {
    /// Block until the provided limit is reached
    None,
    /// Wait at most for the given duration or until the limit is reached
    TotalDuration(Duration),
    /// Wait for the first item for at most the first duration, allow additional items
    /// to be collected for an additional timeframe of the second duration (starting from
    /// the reception of the first item).
    Split(Duration, Duration),
}

/// Structure to wait for and collect one or more raw responses
#[async_trait]
pub trait RawResponseCollector {
    /// Streams one or more raw responses until the given timeout or limit is reached
    async fn collect_raw(
        &self,
        location: ResponseLocation,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<BoxStream<Result<Vec<u8>, BoxedError>>, BoxedError>;
}

/// Collector of typed responses to [`Requests`](super::Request)
#[async_trait]
pub trait ResponseCollector {
    /// Streams one or more deserialized responses until the given timeout or limit is reached
    async fn collect<R: DeserializeOwned + Send + Sync>(
        &self,
        location: ResponseLocation,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<BoxStream<Result<R, BoxedError>>, BoxedError>;
}
