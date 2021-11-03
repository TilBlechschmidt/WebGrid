use super::handle::HandleRegistration;
use crate::library::communication::implementation::redis::{PubSubResource, PubSubResourceError};
use async_trait::async_trait;
use futures::stream::{once, BoxStream};
use futures::StreamExt;
use redis::aio::{Connection, PubSub};
use redis::{Msg, RedisResult};

/// Redis PubSub connection monitoring the connection state
pub struct MonitoredPubSub {
    pubsub: PubSub,
    handle: HandleRegistration,
}

impl MonitoredPubSub {
    pub(super) fn new(con: Connection, handle: HandleRegistration) -> Self {
        Self {
            pubsub: con.into_pubsub(),
            handle,
        }
    }
}

#[async_trait]
impl PubSubResource for MonitoredPubSub {
    async fn psubscribe(&mut self, pchannel: &str) -> RedisResult<()> {
        self.pubsub.psubscribe(pchannel).await
    }

    async fn subscribe(&mut self, channel: &str) -> RedisResult<()> {
        self.pubsub.subscribe(channel).await
    }

    fn into_on_message<'a>(self) -> BoxStream<'a, Result<Msg, PubSubResourceError>> {
        let mut handle = self.handle.clone();

        let message_stream = self
            .pubsub
            .into_on_message()
            .map(Ok::<Msg, PubSubResourceError>);
        let error_stream = once(async move {
            handle.resource_died().await;
            Err(PubSubResourceError::StreamClosed)
        })
        .boxed();

        message_stream.chain(error_stream).boxed()
    }
}
