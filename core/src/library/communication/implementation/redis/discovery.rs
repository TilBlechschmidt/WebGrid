use super::{PubSubResource, RedisFactory};
use crate::library::communication::discovery::pubsub::PubSubServiceDiscoveryBackend;
use crate::library::communication::discovery::{
    ServiceAdvertiser, ServiceAnnouncement, ServiceDescriptor, ServiceEndpoint,
};
use crate::library::communication::implementation::redis::RedisConnectionVariant;
use crate::library::{BoxedError, EmptyResult};
use async_trait::async_trait;
use futures::{
    stream::{self, BoxStream},
    StreamExt,
};
use futures::{Future, TryStreamExt};
use redis::aio::ConnectionLike;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, trace, warn};

const SERVICE_DISCOVERY_CHANNEL_PREFIX: &str = "wg-sd-";
const SERVICE_ANNOUNCEMENT_CHANNEL: &str = "wg-sa";

/// [`ServiceAdvertiser`] implementation based on Redis PubSub channels
pub struct RedisServiceAdvertiser<F: RedisFactory> {
    factory: F,
}

impl<F> RedisServiceAdvertiser<F>
where
    F: RedisFactory,
{
    /// Creates a new instance from an existing redis connection
    pub fn new(factory: F) -> Self {
        Self { factory }
    }
}

#[async_trait]
impl<F> ServiceAdvertiser for RedisServiceAdvertiser<F>
where
    F: RedisFactory + Send + Sync,
    F::PubSub: Send + Sync,
{
    #[instrument(skip(self, on_ready, service), fields(service = std::any::type_name::<S>()))]
    async fn advertise<
        S: ServiceDescriptor + Serialize + Send + Sync,
        Fut: Future<Output = ()> + Send + Sync,
        Fn: FnOnce() -> Fut + Send + Sync,
    >(
        &self,
        service: S,
        endpoint: ServiceEndpoint,
        on_ready: Option<Fn>,
    ) -> EmptyResult {
        debug!("Acquiring advertising connection");
        let mut sub = self.factory.pubsub().await?;
        let mut con = self
            .factory
            .connection(RedisConnectionVariant::Multiplexed)
            .await?;

        let request_channel = format!(
            "{}{}",
            SERVICE_DISCOVERY_CHANNEL_PREFIX,
            service.service_identifier()
        );

        debug!("Serializing service announcement");
        let announcement = ServiceAnnouncement { service, endpoint };
        let message = serde_json::to_string(&announcement)?;

        trace!("Subscribing to advertising request channel");
        sub.subscribe(&request_channel).await?;

        // Preemptively send out an announcement for unsolicited passive caching
        trace!("Preemptively publishing service announcement");
        con.publish(SERVICE_ANNOUNCEMENT_CHANNEL, &message).await?;

        let mut stream = sub.into_on_message();

        trace!("Indicating ready state");
        if let Some(on_ready) = on_ready {
            (on_ready)().await;
        }

        debug!("Entering advertisement loop");
        while stream.try_next().await?.is_some() {
            // To reduce CPU load, we ignorantly assume that *any* message on the request channel will be a request for us.
            // Thus we can skip all the deserialization and just announce our presence (it is an idempotent operation anyways).
            trace!("Publishing advertisement");
            con.publish(SERVICE_ANNOUNCEMENT_CHANNEL, &message).await?;
        }

        Ok(())
    }
}

/// [`PubSubServiceDiscoveryBackend`] implementation based on Redis PubSub channels
pub struct RedisPubSubBackend<F: RedisFactory> {
    factory: F,
    query_con: Arc<Mutex<Box<dyn ConnectionLike + Send + Sync>>>,
}

impl<F> RedisPubSubBackend<F>
where
    F: RedisFactory,
{
    /// Creates a new instance from an existing redis connection
    pub async fn new(factory: F) -> Result<Self, BoxedError> {
        let query_con = Arc::new(Mutex::new(
            factory.connection(RedisConnectionVariant::Owned).await?,
        ));

        Ok(Self { factory, query_con })
    }
}

#[async_trait]
impl<D, F> PubSubServiceDiscoveryBackend<D> for RedisPubSubBackend<F>
where
    F: RedisFactory + Send + Sync,
    F::PubSub: Send + Sync,
    D: ServiceDescriptor + Send + Sync + Serialize + DeserializeOwned,
{
    #[instrument(skip(self))]
    async fn listen<'a>(&self) -> BoxStream<'a, ServiceAnnouncement<D>>
    where
        D: 'a,
    {
        trace!("Acquiring connection");
        let mut connection = match self.factory.pubsub().await {
            Ok(con) => con,
            Err(error) => {
                error!(
                    ?error,
                    "Encountered error creating service discovery listening connection"
                );
                return stream::empty().boxed();
            }
        };

        trace!("Subscribing to service announcement channel");
        if let Err(error) = connection.subscribe(SERVICE_ANNOUNCEMENT_CHANNEL).await {
            error!(?error, "Unable to subscribe to service discovery channel");
            return stream::empty().boxed();
        }

        trace!("Creating service announcement stream");
        let stream = connection
            .into_on_message()
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| async move {
                match x {
                    Ok(message) => {
                        trace!("Parsing incoming service announcement");
                        let payload = message.get_payload_bytes();
                        let announcement =
                            serde_json::from_slice::<ServiceAnnouncement<D>>(payload);

                        match announcement {
                            Ok(announcement) => Some(announcement),
                            Err(error) => {
                                warn!(?error, "Failed to decode incoming announcement");
                                None
                            }
                        }
                    }
                    Err(_) => unreachable!(),
                }
            })
            .boxed();

        stream
    }

    #[instrument(skip(self, descriptor), fields(service = ?descriptor.service_identifier()))]
    async fn query(&self, descriptor: &D) {
        let message = match serde_json::to_vec(&descriptor) {
            Ok(message) => message,
            Err(error) => {
                error!(?error, "Failed to serialize service discovery query");
                return;
            }
        };

        let channel = format!(
            "{}{}",
            SERVICE_DISCOVERY_CHANNEL_PREFIX,
            descriptor.service_identifier()
        );

        trace!(?channel, "Publishing service discovery query");
        if let Err(error) = self
            .query_con
            .lock()
            .await
            .publish::<_, _, ()>(channel, message)
            .await
        {
            error!(?error, "Failed to send service discovery query")
        }
    }
}
