use super::super::{structs::NodeError, Context};
use helpers::{keys, Timeout};
use lifecycle::{
    logging::{LogCode, SessionLogger},
    Heart, HeartStone,
};
use redis::AsyncCommands;
use resources::{with_redis_resource, ResourceManager};
use scheduling::TaskManager;
use std::time::Duration;
use storage_lib::StorageHandler;

pub async fn initialize_service(
    manager: TaskManager<Context>,
) -> Result<(Heart, HeartStone), NodeError> {
    let mut con = with_redis_resource!(manager);

    let (heart, heart_stone) = Heart::with_lifetime(Duration::from_secs(
        Timeout::SessionTermination.get(&mut con).await as u64,
    ));

    let storage_id = StorageHandler::storage_id(manager.context.options.storage_directory.clone())
        .await
        .map_err(|_| NodeError::StorageUnavailable)?;
    con.set::<_, _, ()>(keys::session::storage(&manager.context.id), storage_id)
        .await
        .map_err(|_| NodeError::StorageUnavailable)?;

    let external_session_id: String = manager.context.id.clone();
    let mut logger = SessionLogger::new(con, "node".to_string(), external_session_id.clone());
    logger.log(LogCode::BOOT, None).await.ok();

    Ok((heart, heart_stone))
}
