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

        let announcement = ServiceAnnouncement { service, endpoint };
        let message = serde_json::to_string(&announcement)?;

        sub.subscribe(&request_channel).await?;

        // Preemptively send out an announcement for unsolicited passive caching
        con.publish(SERVICE_ANNOUNCEMENT_CHANNEL, &message).await?;

        let mut stream = sub.into_on_message();

        if let Some(on_ready) = on_ready {
            (on_ready)().await;
        }

        while stream.try_next().await?.is_some() {
            // To reduce CPU load, we ignorantly assume that *any* message on the request channel will be a request for us.
            // Thus we can skip all the deserialization and just announce our presence (it is an idempotent operation anyways).
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
    async fn listen<'a>(&self) -> BoxStream<'a, ServiceAnnouncement<D>>
    where
        D: 'a,
    {
        let mut connection = match self.factory.pubsub().await {
            Ok(con) => con,
            Err(e) => {
                log::error!(
                    "Encountered error creating service discovery listening connection: {}",
                    e
                );
                return stream::empty().boxed();
            }
        };

        if let Err(e) = connection.subscribe(SERVICE_ANNOUNCEMENT_CHANNEL).await {
            log::error!("Unable to subscribe to service discovery channel: {}", e);
            return stream::empty().boxed();
        }

        let stream = connection
            .into_on_message()
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| async move {
                match x {
                    Ok(message) => {
                        // TODO
                        let payload = message.get_payload_bytes();
                        let announcement =
                            serde_json::from_slice::<ServiceAnnouncement<D>>(payload);

                        match announcement {
                            Ok(announcement) => Some(announcement),
                            Err(e) => {
                                log::warn!("Failed to decode incoming announcement: {}", e);
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

    async fn query(&self, descriptor: &D) {
        let message = match serde_json::to_vec(&descriptor) {
            Ok(message) => message,
            Err(e) => {
                log::error!("Failed to serialize service discovery query: {}", e);
                return;
            }
        };

        let channel = format!(
            "{}{}",
            SERVICE_DISCOVERY_CHANNEL_PREFIX,
            descriptor.service_identifier()
        );

        if let Err(e) = self
            .query_con
            .lock()
            .await
            .publish::<_, _, ()>(channel, message)
            .await
        {
            log::error!("Failed to send service discovery query: {}", e)
        }
    }
}
