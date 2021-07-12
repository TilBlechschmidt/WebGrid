//! Serialization and deserialization provided by [`serde_json`] using marker traits
//!
//! This module allows implementors of traits that allow raw access to underlying messaging systems
//! to provide the higher-level traits relying on serialization. It does so by providing a number of
//! marker traits which, when implemented, provide default implementations of the higher-level traits
//! by translating between lower-level serialized data and higher-level strongly typed data by using
//! [`serde_json`]. In the future, this will allow for an easy exchange of serialization algorithms by
//! changing the marker traits.

use super::super::event::{
    Notification, NotificationPublisher, QueueDescriptorExtension, QueueEntry,
    RawNotificationPublisher, RawQueueEntry,
};
use super::super::request::{
    RawResponseCollector, RawResponsePublisher, ResponseCollectionTimeout, ResponseCollector,
    ResponseLocation, ResponsePublisher,
};
use crate::library::{BoxedError, EmptyResult};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Marker trait providing a default [`NotificationPublisher`] implementation based on [`serde_json`]
pub trait JsonNotificationPublisher: RawNotificationPublisher + Send + Sync {}

#[async_trait]
impl<P> NotificationPublisher for P
where
    P: JsonNotificationPublisher,
{
    /// Serializes the notification using [`serde_json::to_string`]
    async fn publish<N: Notification + Send + Sync>(&self, notification: &N) -> EmptyResult {
        let data = serde_json::to_string(notification)?;
        self.publish_raw(data.as_bytes(), N::queue(), None).await
    }

    /// Serializes the notification using [`serde_json::to_string`]
    async fn publish_with_extension<N: Notification + Send + Sync>(
        &self,
        notification: &N,
        extension: QueueDescriptorExtension,
    ) -> EmptyResult {
        let data = serde_json::to_string(notification)?;
        self.publish_raw(data.as_bytes(), N::queue(), Some(extension))
            .await
    }
}

/// Marker trait providing a default [`QueueEntry`] implementation based on [`serde_json`]
pub trait JsonQueueEntry: RawQueueEntry {}

impl<E> QueueEntry for E
where
    E: JsonQueueEntry,
{
    /// Parses the payload using [`serde_json::from_slice`]
    fn parse_payload<'a, T>(&'a self) -> Result<T, BoxedError>
    where
        T: Deserialize<'a>,
    {
        serde_json::from_slice(self.payload()).map_err(Into::into)
    }
}

/// Marker trait providing a default [`ResponseCollector`] implementation based on [`serde_json`]
pub trait JsonResponseCollector: RawResponseCollector + Send + Sync {}

#[async_trait]
impl<C> ResponseCollector for C
where
    C: JsonResponseCollector,
{
    /// Parses the payload using [`serde_json::from_slice`]
    async fn collect<R: DeserializeOwned + Send + Sync>(
        &self,
        location: ResponseLocation,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<BoxStream<Result<R, BoxedError>>, BoxedError> {
        let stream = self
            .collect_raw(location, limit, timeout)
            .await?
            .and_then(|bytes| async move { serde_json::from_slice(&bytes).map_err(Into::into) })
            .boxed();

        Ok(stream)
    }
}

/// Marker trait providing a default [`ResponsePublisher`] implementation based on [`serde_json`]
pub trait JsonResponsePublisher: RawResponsePublisher + Send + Sync {}

#[async_trait]
impl<P> ResponsePublisher for P
where
    P: JsonResponsePublisher,
{
    /// Serializes the response using [`serde_json::to_string`]
    async fn publish<R: Send + Sync + Serialize>(
        &self,
        response: &R,
        location: ResponseLocation,
    ) -> EmptyResult {
        let data = serde_json::to_string(response)?;
        self.publish_raw(data.as_bytes(), location).await
    }
}
