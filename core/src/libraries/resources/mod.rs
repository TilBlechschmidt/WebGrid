//! Monitored resources for jobs
//!
//! This crate provides resources that are monitoring their internal state.
//! If they become unavailable they report this to a TaskHandle provided by the `scheduling` module.
//!
//! Usually this results in the job holding the resource being terminated and restarted.

mod manager;
mod redis;
mod traits;

pub use self::manager::DefaultResourceManager;
pub use self::redis::{RedisResource, SharedRedisResource, StandaloneRedisResource};
pub use traits::{
    PubSub, PubSubResource, PubSubResourceError, ResourceManager, ResourceManagerProvider,
    ResourceManagerResult,
};

/// Shorthand to request a redis resource from a manager
///
/// Requires the context to contain an `impl ResourceManager` named `.resource_manager`.
#[macro_export]
macro_rules! with_redis_resource {
    ($manager:expr) => {
        $manager
            .context
            .resource_manager()
            .redis($manager.create_resource_handle())
            .await
            .expect("Unable to create redis resource")
    };
}

/// Shorthand to request a shared redis resource from a manager
///
/// Requires the context to contain an `impl ResourceManager` named `.resource_manager`.
#[macro_export]
macro_rules! with_shared_redis_resource {
    ($manager:expr) => {
        $manager
            .context
            .resource_manager()
            .shared_redis($manager.create_resource_handle())
            .await
            .expect("Unable to create redis resource")
    };
}
