use super::super::super::BoxedError;
use super::super::event::NotificationPublisher;
use super::ResponseCollectionTimeout;
use super::{Request, ResponseCollector};
use async_trait::async_trait;
use futures::TryStreamExt;
use thiserror::Error;

/// Error type for sending requests
#[derive(Error, Debug)]
pub enum RequestError {
    /// Publishing of the request failed
    #[error("sending of request failed")]
    SendingFailure(#[source] BoxedError),
    /// Response collector was unable to start listening for responses
    #[error("unable to collect responses")]
    ResponseCollectionFailed(#[source] BoxedError),
    /// An individual response could not be received or parsed
    #[error("response not receivable")]
    ReceptionFailed(#[source] BoxedError),
}

/// Handler for sending requests and collecting responses
#[async_trait]
pub trait Requestor {
    /// Sends out a request and awaits responses
    ///
    /// Note that either a `limit` or `timeout` has to be provided. If neither is given, the function would block indefinitely and will panic.
    async fn request<R>(
        &self,
        request: &R,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<Vec<R::Response>, RequestError>
    where
        R: Request + Send + Sync,
        R::Response: Send + Sync;
}

/// [`Requestor`] implementation by combining a [`NotificationPublisher`] and [`ResponseCollector`]
pub struct CompositeRequestor<P: NotificationPublisher, C: ResponseCollector> {
    publisher: P,
    collector: C,
}

impl<P, C> CompositeRequestor<P, C>
where
    P: NotificationPublisher,
    C: ResponseCollector,
{
    /// Creates a new instance from raw parts
    pub fn new(publisher: P, collector: C) -> Self {
        Self {
            publisher,
            collector,
        }
    }
}

#[async_trait]
impl<P, C> Requestor for CompositeRequestor<P, C>
where
    P: NotificationPublisher + Send + Sync,
    C: ResponseCollector + Send + Sync,
{
    /// Sends a request by delegating to a [`NotificationPublisher`] and collects responses using a [`ResponseCollector`]
    async fn request<R>(
        &self,
        request: &R,
        limit: Option<usize>,
        timeout: ResponseCollectionTimeout,
    ) -> Result<Vec<R::Response>, RequestError>
    where
        R: Request + Send + Sync,
        R::Response: Send + Sync,
    {
        assert!(
            limit.is_some() || timeout != ResponseCollectionTimeout::None,
            "Calling `request` without a limit or timeout would block indefinitely!"
        );

        // Send the request
        self.publisher
            .publish(request)
            .await
            .map_err(|e| RequestError::SendingFailure(e))?;

        // Create a stream for receiving responses
        let stream = self
            .collector
            .collect::<R::Response>(request.reply_to(), limit, timeout)
            .await
            .map_err(|e| RequestError::ResponseCollectionFailed(e))?;

        // Condense the stream of responses and flatten the errors
        // TODO Contemplate whether a single erroneous response should poison all the other responses
        //      Pro: Makes the error explicit and propagates the fact that there is an issue instead of it rotting in some log
        //      Con: Reduces the resilience as the service may have been able to continue operation by just using the remaining, valid responses
        let responses = stream
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| RequestError::ReceptionFailed(e))?;

        Ok(responses)
    }
}
