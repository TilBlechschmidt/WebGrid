//! Service discovery and advertisement structures

use crate::library::EmptyResult;

use super::super::BoxedError;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

pub mod pubsub;

/// Entity which describes a service that can be discovered
pub trait ServiceDescriptor: Clone {
    /// Unique identifier of the service type or instance
    fn service_identifier(&self) -> String;
}

/// Individual endpoint for some service with the optional
/// ability to mark it as unreachable.
#[async_trait]
pub trait DiscoveredServiceEndpoint: Deref<Target = ServiceEndpoint> {
    /// Marks the endpoint as stale or otherwise unusable so that
    /// future discovery attempts may attempt a different route.
    async fn flag_unreachable(&self);
}

/// Structure used to discover endpoints for services
#[async_trait]
pub trait ServiceDiscoverer<D: ServiceDescriptor> {
    /// Implementation type for the [`DiscoveredServiceEndpoint`]
    type I: DiscoveredServiceEndpoint;

    /// Starts the discovery process for a given service. The resulting stream
    /// yields [`DiscoveredServiceEndpoint`] instances or an error if no more endpoints are available.
    /// In case the consumer finds that one of the returned endpoints is unreachable, it should flag it
    /// as such so that future discovery attempts will not attempt to re-use the same endpoint (although
    /// the specifics are up to the implementation of this trait).
    fn discover<'a>(&self, descriptor: D) -> BoxStream<'a, Result<Self::I, BoxedError>>
    where
        D: 'a;
}

/// Domain specific description of where a service is reachable
pub type ServiceEndpoint = String;

/// Structure advertising a service
#[async_trait]
pub trait ServiceAdvertiser {
    /// Advertises the given job while the returned future is polled
    async fn advertise<S: ServiceDescriptor + Serialize + Send + Sync>(
        &self,
        service: S,
        endpoint: ServiceEndpoint,
    ) -> EmptyResult;
}

/// Generic implementation of a service announcement that may be used by
/// implementations of the [`ServiceAdvertiser`] and [`ServiceDiscoverer`].
/// Does nothing by itself and is only a shell carrying information.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceAnnouncement<D: ServiceDescriptor> {
    /// Service identification
    pub service: D,
    /// Endpoint where the service is reachable
    pub endpoint: ServiceEndpoint,
}

impl<D: ServiceDescriptor> ServiceAnnouncement<D> {
    /// Creates a new announcement from raw parts
    pub fn new(service: D, endpoint: ServiceEndpoint) -> Self {
        Self { service, endpoint }
    }
}
