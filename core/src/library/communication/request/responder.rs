use super::super::super::BoxedError;
use super::super::super::EmptyResult;
use super::super::event::Consumer;
use super::Request;
use super::ResponsePublisher;
use async_trait::async_trait;
use thiserror::Error;

/// Error that may be thrown while responding to a request
#[derive(Error, Debug)]
pub enum ResponderError {
    /// The [`RequestProcessor`] threw an error
    #[error("processing request failed")]
    ProcessingFailed(#[source] BoxedError),
    /// Unable to deliver the response
    #[error("sending response failed")]
    ResponseUndeliverable(#[source] BoxedError),
}

/// Structure which processes requests and produces responses
#[async_trait]
pub trait RequestProcessor {
    /// Type of request to process
    type Request: Request;

    /// Handler for requests, returning a response
    async fn process(
        &self,
        request: Self::Request,
    ) -> Result<<Self::Request as Request>::Response, BoxedError>;
}

/// Structure which processes requests and may either produce a response or ignore the request
#[async_trait]
pub trait OptionalRequestProcessor {
    /// Type of request to process
    type Request: Request;

    /// Handler for requests, returning an optional response
    async fn maybe_process(
        &self,
        request: Self::Request,
    ) -> Result<Option<<Self::Request as Request>::Response>, BoxedError>;
}

#[async_trait]
impl<P> OptionalRequestProcessor for P
where
    P: RequestProcessor + Send + Sync,
    P::Request: Send + Sync,
{
    type Request = P::Request;

    async fn maybe_process(
        &self,
        request: Self::Request,
    ) -> Result<Option<<Self::Request as Request>::Response>, BoxedError> {
        Ok(Some(self.process(request).await?))
    }
}

/// Convenience wrapper to process requests and send responses
pub struct Responder<R, C: OptionalRequestProcessor<Request = R>, P> {
    processor: C,
    publisher: P,
}

impl<R, C, P> Responder<R, C, P>
where
    R: Request,
    C: OptionalRequestProcessor<Request = R>,
    P: ResponsePublisher,
{
    /// Creates a new responder from raw parts
    pub fn new(processor: C, publisher: P) -> Self {
        Self {
            processor,
            publisher,
        }
    }
}

#[async_trait]
impl<R, C, P> Consumer for Responder<R, C, P>
where
    R: Request + Send + Sync,
    R::Response: Send + Sync,
    C: OptionalRequestProcessor<Request = R> + Send + Sync,
    P: ResponsePublisher + Send + Sync,
{
    type Notification = R;

    async fn consume(&self, request: Self::Notification) -> EmptyResult {
        let location = request.reply_to();

        // TODO Contemplate whether a message should not be acknowledged due to a processing failure
        //      Pro: Retrying it later may succeed (but requires the service to be restarted)
        //      Con: Potentially needlessly occupies the PEL of the consumer if the failure is caused by the request being invalid
        //      Idea: A processor may only return Err(_) when a system failure occurs. For subject specific failures, a response
        //              containing a Result should be used instead!
        if let Some(response) = self
            .processor
            .maybe_process(request)
            .await
            .map_err(|e| ResponderError::ProcessingFailed(e))?
        {
            self.publisher
                .publish(&response, location)
                .await
                .map_err(|e| ResponderError::ResponseUndeliverable(e))?;
        }

        Ok(())
    }
}
