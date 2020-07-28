use super::super::{structs::NodeError, Context};
use crate::libraries::helpers::lua;
use crate::libraries::resources::ResourceManager;
use crate::libraries::scheduling::TaskManager;
use crate::with_redis_resource;
use chrono::offset::Utc;
use redis::Script;

pub async fn terminate(manager: TaskManager<Context>) -> Result<(), NodeError> {
    let mut con = with_redis_resource!(manager);

    let script_content = format!(
        r#"
        local sessionID = ARGV[1];
        local currentTime = ARGV[2];
        {loadOrchestratorID}
        {terminateSession}
    "#,
        loadOrchestratorID = lua::fetch_orchestrator_from_session(),
        terminateSession = lua::terminate_session(),
    );

    let script = Script::new(&script_content);
    let _: Option<()> = script
        .arg(manager.context.id)
        .arg(Utc::now().to_rfc3339())
        .invoke_async(&mut con)
        .await
        .ok();

    Ok(())
}
