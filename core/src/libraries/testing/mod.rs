mod harness;
mod lock_manager;
pub mod resources;
pub mod setup;

// TODO Most of these tests should logically belong into the resources crate, however that would create a cyclic dependency.
// #[cfg(test)]
// mod tests {
//     use super::*;

//     use crate::libraries::resources::{PubSub, ResourceManager};
//     use crate::libraries::scheduling::TaskResourceHandle;
//     use futures::stream::TryStreamExt;
//     use redis::AsyncCommands;

//     #[test]
//     fn unique_identifier_are_unique() {
//         let identifier_a = unique_identifier!();
//         let identifier_b = unique_identifier!();

//         assert_ne!(identifier_a, identifier_b);
//     }

//     #[test]
//     fn pubsub_test() {
//         let channel_name = unique_identifier!();
//         let payload = "test";

//         with_resource_manager!(manager, {
//             let redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();
//             let mut redis2 = manager.redis(TaskResourceHandle::stub()).await.unwrap();

//             let mut pubsub: PubSub = redis.into();
//             pubsub.psubscribe(channel_name).await.unwrap();
//             let mut stream = pubsub.on_message();

//             redis2
//                 .publish::<_, _, ()>(channel_name, payload)
//                 .await
//                 .unwrap();

//             if let Ok(Some(msg)) = stream.try_next().await {
//                 assert_eq!(msg.get_payload::<String>().unwrap(), payload.to_owned());
//             } else {
//                 panic!()
//             }
//         });
//     }

//     #[test]
//     fn redis_ping_set() {
//         with_resource_manager!(manager, {
//             let mut redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();
//             redis::cmd("PING")
//                 .query_async::<_, ()>(&mut redis)
//                 .await
//                 .ok();
//             redis.set::<_, _, ()>("OtherKey", "Blub").await.unwrap();
//         });
//     }

//     #[test]
//     fn redis_does_not_collide() {
//         // This test would fail in the setup if another test would've modified the same database
//         with_resource_manager!(manager, {
//             let mut redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();

//             redis.set::<_, _, ()>("OtherKey", "Blub").await.unwrap();
//             redis.set::<_, _, ()>("OtherKey", "Blub").await.unwrap();
//         });
//     }

//     #[test]
//     fn load_commands_from_string() {
//         // TODO This test needs to be fully implemented!
//         with_resource_manager!(manager, {
//             let mut redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();

//             let initial_commands = "
//                 SET a 10
//                 SET b 20
//                 HSET test c 30
//                 HSET test d 40
//             ";

//             setup::load(&mut redis, initial_commands).await;

//             let (next_cursor, keys): (String, Vec<String>) = redis::cmd("SCAN")
//                 .arg(0)
//                 .query_async(&mut redis)
//                 .await
//                 .unwrap();

//             println!("Next cursor: {}", next_cursor);
//             println!("{:?}", keys);
//         });
//     }
// }
