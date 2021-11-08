use crate::communication::request::ResponsePublisher;
use async_trait::async_trait;

pub struct MockResponsePublisher {}

#[async_trait]
impl ResponsePublisher for MockResponsePublisher {
    async fn publish<R: Send + Sync + serde::Serialize>(
        &self,
        _response: &R,
        _location: crate::communication::request::ResponseLocation,
    ) -> crate::EmptyResult {
        unimplemented!()
    }
}
