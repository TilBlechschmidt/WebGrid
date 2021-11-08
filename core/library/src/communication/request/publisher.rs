use super::super::super::EmptyResult;
use super::ResponseLocation;
use async_trait::async_trait;
use serde::Serialize;

/// Structure which allows publishing of raw responses
#[async_trait]
pub trait RawResponsePublisher {
    /// Appends an opaque payload to a [`ResponseLocation`] list structure
    async fn publish_raw(&self, data: &[u8], location: ResponseLocation) -> EmptyResult;
}

/// Publisher for responses to [`Requests`](super::Request)
#[async_trait]
pub trait ResponsePublisher {
    /// Appends a response to a [`ResponseLocation`] list structure
    async fn publish<R: Send + Sync + Serialize>(
        &self,
        response: &R,
        location: ResponseLocation,
    ) -> EmptyResult;
}
