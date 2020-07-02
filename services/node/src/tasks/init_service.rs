use crate::{structs::NodeError, Context};
use helpers::{env, Timeout};
use lifecycle::{
    logging::{LogCode, SessionLogger},
    Heart, HeartStone,
};
use resources::{with_redis_resource, ResourceManager};
use scheduling::TaskManager;
use std::time::Duration;

pub async fn initialize_service(
    manager: TaskManager<Context>,
) -> Result<(Heart, HeartStone), NodeError> {
    let mut con = with_redis_resource!(manager);

    let (heart, heart_stone) = Heart::with_lifetime(Duration::from_secs(
        Timeout::SessionTermination.get(&mut con).await as u64,
    ));

    let external_session_id: String = (*env::service::node::ID).clone();
    let mut logger = SessionLogger::new(con, "node".to_string(), external_session_id.clone());
    logger.log(LogCode::BOOT, None).await.ok();

    Ok((heart, heart_stone))
}
