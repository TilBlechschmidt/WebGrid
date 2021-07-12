//! PubSub channel based discovery implementation
//!
//! This module contains types which provide a high-level wrapper around an underlying PubSub messaging channel.
//! It implements concepts like caching and passive service discovery while delegating network functionality to
//! an implementation of the [`PubSubServiceDiscoveryBackend`] trait.

use super::*;
use futures::select;
use futures::{stream::unfold, FutureExt, StreamExt};
use lru::LruCache;
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::{
    pin,
    sync::{broadcast, mpsc, Mutex},
    time::{sleep_until, Instant},
};

const MAX_DISCOVERY_RETRIES: u8 = 4;
const DISCOVERY_REQUEST_TIMEOUT: Duration = Duration::from_millis(500);

type ServiceEndpointCache<Descriptor> = Arc<Mutex<LruCache<Descriptor, HashSet<ServiceEndpoint>>>>;

/// Network backend for PubSub based service discovery
#[async_trait]
pub trait PubSubServiceDiscoveryBackend<D: ServiceDescriptor> {
    /// Streams incoming [`ServiceAnnouncements`](ServiceAnnouncement)
    async fn listen<'a>(&self) -> BoxStream<'a, ServiceAnnouncement<D>>
    where
        D: 'a;
    /// Sends out a discovery request for a specific service
    async fn query(&self, descriptor: &D);
}

/// Background daemon for discovering service using PubSub channels
pub struct PubSubServiceDiscovererDaemon<D: ServiceDescriptor> {
    cache: ServiceEndpointCache<D>,
    request_rx: Arc<Mutex<mpsc::Receiver<D>>>,
    response_tx: broadcast::Sender<ServiceAnnouncement<D>>,
}

impl<D> PubSubServiceDiscovererDaemon<D>
where
    D: ServiceDescriptor + Eq + Hash,
{
    /// Main loop which handles outgoing service queries and incoming announcements
    /// as well as cache operations.
    pub async fn daemon_loop<B>(&self, backend: B)
    where
        B: PubSubServiceDiscoveryBackend<D>,
    {
        let mut response_stream = backend.listen().await.fuse();
        let mut request_rx = self
            .request_rx
            .try_lock()
            .expect("Attempted to run more than one PubSubServiceDiscovererDaemon instances simultaneously!");

        loop {
            select! {
                request = request_rx.recv().fuse() => {
                    match request {
                        Some(request) => backend.query(&request).await,
                        None => break
                    }
                }
                response = response_stream.next() => {
                    match response {
                        Some(response) => self.handle_response(response).await,
                        None => break
                    }
                }
            }
        }
    }

    async fn handle_response(&self, response: ServiceAnnouncement<D>) {
        // Insert the value into the cache
        let mut cache = self.cache.lock().await;
        if let Some(entry) = cache.get_mut(&response.service) {
            entry.insert(response.endpoint.clone());
        } else {
            let mut set = HashSet::new();
            set.insert(response.endpoint.clone());
            cache.put(response.service.clone(), set);
        }

        // Notify anybody waiting for the response
        self.response_tx.send(response).ok();
    }
}

/// [`ServiceDiscoverer`] implementation based on a PubSub messaging channel
///
/// Internally, it provides a cache and fills it through passive listening.
#[derive(Clone)]
pub struct PubSubServiceDiscoverer<D: ServiceDescriptor> {
    cache: ServiceEndpointCache<D>,
    request_tx: mpsc::Sender<D>,
    response_tx: broadcast::Sender<ServiceAnnouncement<D>>,
}

impl<D> PubSubServiceDiscoverer<D>
where
    D: ServiceDescriptor + Eq + Hash + std::fmt::Debug + Send + Sync,
{
    /// Creates a new PubSub based [`ServiceDiscoverer`] and a linked daemon instance
    pub fn new(
        cache_size: usize,
        request_channel_size: usize,
        response_channel_size: usize,
    ) -> (Self, PubSubServiceDiscovererDaemon<D>) {
        let cache = Arc::new(Mutex::new(LruCache::new(cache_size)));
        let (request_tx, request_rx) = mpsc::channel(request_channel_size);
        let (response_tx, _) = broadcast::channel(response_channel_size);

        let discoverer = Self {
            cache: cache.clone(),
            request_tx,
            response_tx: response_tx.clone(),
        };

        let daemon = PubSubServiceDiscovererDaemon {
            cache,
            request_rx: Arc::new(Mutex::new(request_rx)),
            response_tx,
        };

        (discoverer, daemon)
    }
}

impl<D> ServiceDiscoverer<D> for PubSubServiceDiscoverer<D>
where
    D: ServiceDescriptor + Eq + Hash + std::fmt::Debug + Send + Sync,
{
    type I = PubSubDiscoveredServiceEndpoint<D>;

    fn discover<'a>(&self, descriptor: D) -> BoxStream<'a, Result<Self::I, BoxedError>>
    where
        D: 'a,
    {
        let workflow = ServiceDiscoveryWorkflow {
            service: descriptor,
            cache: self.cache.clone(),

            response_rx: self.response_tx.subscribe(),
            request_tx: self.request_tx.clone(),

            retries: 0,
            max_retries: MAX_DISCOVERY_RETRIES,
        };

        unfold(workflow, |mut workflow| async move {
            match workflow.discover().await {
                Ok(endpoint) => Some((Ok(endpoint), workflow)),
                Err(ServiceDiscoveryError::RetriesExceeded) => None,
                Err(e) => {
                    let error: BoxedError = e.into();
                    Some((Err(error), workflow))
                }
            }
        })
        .boxed()
    }
}

/// Data container for one discovery operation. Used within the `discover` method of the [`PubSubServiceDiscoverer`].
struct ServiceDiscoveryWorkflow<D: ServiceDescriptor> {
    service: D,
    cache: ServiceEndpointCache<D>,

    response_rx: broadcast::Receiver<ServiceAnnouncement<D>>,
    request_tx: mpsc::Sender<D>,

    retries: u8,
    max_retries: u8,
}

#[derive(Error, Debug)]
enum ServiceDiscoveryError {
    #[error("maximum number of discovery retries exceeded")]
    RetriesExceeded,
    #[error("discovery service channel disconnected")]
    Disconnect(#[from] broadcast::error::RecvError),
    #[error("timed out waiting for discovery")]
    Timeout,
    #[error("unable to send request")]
    RequestFailed,
}

impl<D> ServiceDiscoveryWorkflow<D>
where
    D: ServiceDescriptor + Eq + Hash + std::fmt::Debug + Send + Sync,
{
    /// Attempts to discover an endpoint
    ///
    /// Starts by looking at the cache, if that fails it sends out a discovery request.
    /// If no response is received within a certain timeframe, the process repeats.
    /// When the cache is empty and multiple active discovery attempts have been made, an error is returned.
    async fn discover(
        &mut self,
    ) -> Result<PubSubDiscoveredServiceEndpoint<D>, ServiceDiscoveryError> {
        while self.retries < self.max_retries {
            // Try discovering a new endpoint, retry when we hit a timeout
            // (but increase the number of retries to set an upper limit)
            match self.discover_once().await {
                Ok(endpoint) => {
                    return Ok(PubSubDiscoveredServiceEndpoint {
                        endpoint,
                        service: self.service.clone(),
                        cache: self.cache.clone(),
                    })
                }
                Err(ServiceDiscoveryError::Timeout) => self.retries += 1,
                Err(e) => return Err(e),
            }
        }

        Err(ServiceDiscoveryError::RetriesExceeded)
    }

    async fn discover_once(&mut self) -> Result<ServiceEndpoint, ServiceDiscoveryError> {
        // Try fetching a random element from cache
        if let Some(endpoints) = self.cache.lock().await.get(&self.service) {
            let mut rng = thread_rng();
            if let Some(endpoint) = endpoints.iter().choose(&mut rng) {
                return Ok(endpoint.clone());
            }
        }

        // On cache miss, send out a discovery request
        if self.request_tx.send(self.service.clone()).await.is_err() {
            // There is only ever one possible error cause so no need to pass it on
            return Err(ServiceDiscoveryError::RequestFailed);
        }

        // Wait for a response, but not forever
        let deadline = Instant::now() + DISCOVERY_REQUEST_TIMEOUT;

        loop {
            let message_future = self.response_rx.recv().fuse();
            pin!(message_future);

            let response = select! {
                response = message_future => Some(response),
                _ = sleep_until(deadline).fuse() => None,
            };

            match response {
                None => return Err(ServiceDiscoveryError::Timeout),
                Some(Err(e)) => return Err(e.into()),
                Some(Ok(response)) => {
                    if response.service == self.service {
                        return Ok(response.endpoint);
                    }
                }
            };
        }
    }
}

/// [`DiscoveredServiceEndpoint`] implementation from a [`PubSubServiceDiscoverer`]
pub struct PubSubDiscoveredServiceEndpoint<D> {
    endpoint: ServiceEndpoint,
    service: D,
    cache: ServiceEndpointCache<D>,
}

#[async_trait]
impl<D> DiscoveredServiceEndpoint for PubSubDiscoveredServiceEndpoint<D>
where
    D: ServiceDescriptor + Eq + Hash + Send + Sync,
{
    async fn flag_unreachable(&self) {
        if let Some(endpoints) = self.cache.lock().await.get_mut(&self.service) {
            endpoints.remove(&self.endpoint);
        }
    }
}

impl<D> Deref for PubSubDiscoveredServiceEndpoint<D> {
    type Target = ServiceEndpoint;

    fn deref(&self) -> &Self::Target {
        &self.endpoint
    }
}

// TODO Write tests for the PubSub stuff ;)
