use crate::library::communication::discovery::{
    ServiceAdvertiser, ServiceDescriptor, ServiceEndpoint,
};
use crate::library::EmptyResult;
use async_trait::async_trait;
use serde::Serialize;

pub struct MockServiceAdvertiser {}

#[async_trait]
impl ServiceAdvertiser for MockServiceAdvertiser {
    async fn advertise<S: ServiceDescriptor + Serialize + Send + Sync>(
        &self,
        _service: S,
        _endpoint: ServiceEndpoint,
    ) -> EmptyResult {
        unimplemented!()
    }
}
