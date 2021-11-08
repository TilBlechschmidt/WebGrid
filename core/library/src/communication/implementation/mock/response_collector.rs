use crate::communication::request::ResponseCollector;
use async_trait::async_trait;

pub struct MockResponseCollector {}

#[async_trait]
impl ResponseCollector for MockResponseCollector {
    async fn collect<R: serde::de::DeserializeOwned + Send + Sync>(
        &self,
        _location: crate::communication::request::ResponseLocation,
        _limit: Option<usize>,
        _timeout: crate::communication::request::ResponseCollectionTimeout,
    ) -> Result<futures::stream::BoxStream<Result<R, crate::BoxedError>>, crate::BoxedError> {
        unimplemented!()
    }
}
