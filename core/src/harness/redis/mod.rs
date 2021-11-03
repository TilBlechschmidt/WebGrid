mod discovery;
mod factory;
mod handle;
mod pubsub;
mod resource;

pub use discovery::{RedisServiceAdvertisementJob, RedisServiceDiscoveryJob};
pub use factory::{DummyResourceHandleProvider, RedisCommunicationFactory};
