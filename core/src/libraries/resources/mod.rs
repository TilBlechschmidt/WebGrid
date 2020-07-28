mod manager;
mod redis;
mod traits;

pub use self::manager::DefaultResourceManager;
pub use self::redis::{RedisResource, SharedRedisResource, StandaloneRedisResource};
pub use traits::{
    PubSub, PubSubResource, PubSubResourceError, ResourceManager, ResourceManagerResult,
};

// TODO Write test for shared resource

#[macro_export]
/// Shorthand to request a redis resource from a manager. Requires the context to contain an `impl ResourceManager` named `.resource_manager`.
macro_rules! with_redis_resource {
    ($manager:expr) => {
        $manager
            .context
            .resource_manager
            .redis($manager.create_resource_handle())
            .await
            .expect("Unable to create redis resource")
    };
}

#[macro_export]
/// Shorthand to request a shared redis resource from a manager. Requires the context to contain an `impl ResourceManager` named `.resource_manager`.
macro_rules! with_shared_redis_resource {
    ($manager:expr) => {
        $manager
            .context
            .resource_manager
            .shared_redis($manager.create_resource_handle())
            .await
            .expect("Unable to create redis resource")
    };
}
