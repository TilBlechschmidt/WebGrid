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
use tracing::{debug, instrument, trace, warn};

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

    // Technically, this serves no purpose here. However, to prevent a race-condition
    // in tokio::sync::broadcast, we keep an instance of the response_rx in memory at all times.
    _response_rx: broadcast::Receiver<ServiceAnnouncement<D>>,
}

impl<D> PubSubServiceDiscovererDaemon<D>
where
    D: ServiceDescriptor + Eq + Hash + std::fmt::Debug,
{
    /// Main loop which handles outgoing service queries and incoming announcements
    /// as well as cache operations.
    #[instrument(skip(self, backend), fields(backend = std::any::type_name::<B>()))]
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
                        Some(request) => {
                            trace!(request.service = ?request.service_identifier(), "Received service discovery request");
                            backend.query(&request).await
                        },
                        None => {
                            warn!("Service discovery request stream ended");
                            break
                        }
                    }
                }
                response = response_stream.next() => {
                    match response {
                        Some(response) => {
                            trace!(response.service = ?response.service.service_identifier(), ?response.endpoint, "Received service discovery response");
                            self.handle_response(response).await
                        },
                        None => {
                            warn!("Service discovery response stream ended");
                            break
                        }
                    }
                }
            }
        }
    }

    #[instrument(skip(self, response), fields(response.service = ?response.service.service_identifier(), ?response.endpoint))]
    async fn handle_response(&self, response: ServiceAnnouncement<D>) {
        // Insert the value into the cache
        trace!("Inserting endpoint into cache");
        let mut cache = self.cache.lock().await;
        if let Some(entry) = cache.get_mut(&response.service) {
            entry.insert(response.endpoint.clone());
        } else {
            let mut set = HashSet::new();
            set.insert(response.endpoint.clone());
            cache.put(response.service.clone(), set);
        }

        // Notify anybody waiting for the response
        trace!("Notifying tasks waiting for response");
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
        let (response_tx, response_rx) = broadcast::channel(response_channel_size);

        let discoverer = Self {
            cache: cache.clone(),
            request_tx,
            response_tx: response_tx.clone(),
        };

        let daemon = PubSubServiceDiscovererDaemon {
            cache,
            request_rx: Arc::new(Mutex::new(request_rx)),
            response_tx,
            _response_rx: response_rx,
        };

        (discoverer, daemon)
    }
}

impl<D> ServiceDiscoverer<D> for PubSubServiceDiscoverer<D>
where
    D: ServiceDescriptor + Eq + Hash + std::fmt::Debug + Send + Sync,
{
    type I = PubSubDiscoveredServiceEndpoint<D>;

    #[instrument(skip(self, descriptor), fields(service = ?descriptor.service_identifier()))]
    fn discover<'a>(&self, descriptor: D) -> BoxStream<'a, Result<Self::I, BoxedError>>
    where
        D: 'a,
    {
        debug!("Starting new discovery workflow");
        let workflow = ServiceDiscoveryWorkflow {
            service: descriptor,
            cache: self.cache.clone(),

            response_rx: self.response_tx.subscribe(),
            request_tx: self.request_tx.clone(),

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
    #[instrument(skip(self), fields(service = ?self.service.service_identifier(), max_retries = self.max_retries))]
    async fn discover(
        &mut self,
    ) -> Result<PubSubDiscoveredServiceEndpoint<D>, ServiceDiscoveryError> {
        let mut retries = 0;

        debug!("Starting endpoint discovery loop");

        while retries < self.max_retries {
            debug!(attempt = retries + 1, "Endpoint discovery");
            // Try discovering a new endpoint, retry when we hit a timeout
            // (but increase the number of retries to set an upper limit)
            match self.discover_once().await {
                Ok(endpoint) => {
                    debug!(?endpoint, "Discovery successful");
                    return Ok(PubSubDiscoveredServiceEndpoint {
                        endpoint,
                        service: self.service.clone(),
                        cache: self.cache.clone(),
                    });
                }
                Err(ServiceDiscoveryError::Timeout) => {
                    debug!("Discovery attempt timed out");
                    retries += 1;
                }
                Err(error) => {
                    warn!(?error, "Service discovery failed");
                    return Err(error);
                }
            }
        }

        debug!("Maximum number of service discovery retries exceeded");

        Err(ServiceDiscoveryError::RetriesExceeded)
    }

    #[instrument(skip(self))]
    async fn discover_once(&mut self) -> Result<ServiceEndpoint, ServiceDiscoveryError> {
        // Try fetching a random element from cache
        if let Some(endpoints) = self.cache.lock().await.get(&self.service) {
            let mut rng = thread_rng();
            if let Some(endpoint) = endpoints.iter().choose(&mut rng) {
                trace!(?endpoint, "Cache hit");
                return Ok(endpoint.clone());
            }
        }

        // On cache miss, send out a discovery request
        trace!("Cache miss, sending discovery request");
        self.request_tx
            .send(self.service.clone())
            .await
            .map_err(|_| ServiceDiscoveryError::RequestFailed)?;

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
    #[instrument(skip(self), fields(service = ?self.service.service_identifier()))]
    async fn flag_unreachable(&self) {
        debug!(endpoint = ?self.endpoint, "Flagging endpoint as unreachable");
        if let Some(endpoints) = self.cache.lock().await.get_mut(&self.service) {
            endpoints.remove(&self.endpoint);
        } else {
            warn!("Attempted to flag non-existent endpoint as unreachable");
        }
    }
}

impl<D> Deref for PubSubDiscoveredServiceEndpoint<D> {
    type Target = ServiceEndpoint;

    fn deref(&self) -> &Self::Target {
        &self.endpoint
    }
}

#[cfg(test)]
mod does {
    use futures::TryStreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum ExampleDescriptor {
        SomeService(usize),
    }

    impl ServiceDescriptor for ExampleDescriptor {
        fn service_identifier(&self) -> String {
            match self {
                ExampleDescriptor::SomeService(id) => format!("someservice-{}", id),
            }
        }
    }

    #[derive(Clone)]
    struct EchoBackend<D> {
        tx: broadcast::Sender<D>,
        query_count: Arc<AtomicUsize>,
    }

    impl<D> EchoBackend<D>
    where
        D: ServiceDescriptor + Send + Sync + std::fmt::Debug,
    {
        fn query_count(&self) -> usize {
            self.query_count.load(Ordering::Relaxed)
        }
    }

    impl<D> Default for EchoBackend<D>
    where
        D: ServiceDescriptor + Send + Sync + std::fmt::Debug,
    {
        fn default() -> Self {
            let (tx, _rx) = broadcast::channel(1000);
            Self {
                tx,
                query_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl<D> PubSubServiceDiscoveryBackend<D> for EchoBackend<D>
    where
        D: ServiceDescriptor + Send + Sync + std::fmt::Debug,
    {
        async fn listen<'a>(&self) -> BoxStream<'a, ServiceAnnouncement<D>>
        where
            D: 'a,
        {
            let stream = unfold(self.tx.subscribe(), |mut rx| async move {
                let service = rx.recv().await.unwrap();
                let announcement = ServiceAnnouncement::new(service, "somewhere".into());
                Some((announcement, rx))
            });

            stream.boxed()
        }

        async fn query(&self, descriptor: &D) {
            self.query_count.fetch_add(1, Ordering::Relaxed);
            self.tx.send(descriptor.clone()).unwrap();
        }
    }

    #[tokio::test]
    async fn fulfill_requests() {
        let (discoverer, discovery_daemon) =
            PubSubServiceDiscoverer::<ExampleDescriptor>::new(0, 1, 1);
        let backend = EchoBackend::default();

        tokio::spawn(async move {
            discovery_daemon.daemon_loop(backend).await;
        });

        let mut discovery_stream = discoverer.discover(ExampleDescriptor::SomeService(42));
        let endpoint = discovery_stream.try_next().await.unwrap();
        assert!(endpoint.is_some());
    }

    #[tokio::test]
    async fn cache_responses() {
        let (discoverer, discovery_daemon) =
            PubSubServiceDiscoverer::<ExampleDescriptor>::new(1, 1, 1);
        let backend = EchoBackend::default();
        let cloned_backend = backend.clone();

        tokio::spawn(async move {
            discovery_daemon.daemon_loop(cloned_backend).await;
        });

        for _ in 0..1_000 {
            let mut discovery_stream = discoverer.discover(ExampleDescriptor::SomeService(42));
            let endpoint = discovery_stream.try_next().await.unwrap();
            assert!(endpoint.is_some());
        }

        assert_eq!(backend.query_count(), 1);
    }
}
