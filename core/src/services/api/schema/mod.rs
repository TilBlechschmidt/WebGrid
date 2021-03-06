use crate::libraries::resources::RedisResource;
use juniper::{EmptyMutation, EmptySubscription, RootNode};
pub use query::Query;
use redis::aio::MultiplexedConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

mod query;
mod types;

pub struct GqlContext {
    // TODO Clone the multiplexed connection instead of using a Mutex (it should be more efficient)
    pub redis: Arc<Mutex<RedisResource<MultiplexedConnection>>>,
}

impl juniper::Context for GqlContext {}

pub type Schema =
    RootNode<'static, Query, EmptyMutation<GqlContext>, EmptySubscription<GqlContext>>;

pub fn schema() -> Schema {
    Schema::new(
        Query,
        EmptyMutation::<GqlContext>::new(),
        EmptySubscription::<GqlContext>::new(),
    )
}
