//! Lua functions used for Redis interaction

// TODO Figure out if there is a reasonable way to use crate::keys

/// Lua script to clean up a session object in the redis database
///
/// Returns the associated slot, moves it to the terminated state and removed the heartbeat.
/// The following variables have to be defined for the script to work:
/// - sessionID
/// - orchestrator
/// - currentTime
///
/// These variables are being defined by the script:
/// - slot
pub fn terminate_session() -> String {
    r"
    local slot = redis.call('get', 'session:' .. sessionID .. ':slot')
    redis.call('DEL', 'session:' .. sessionID .. ':slot')
    redis.call('RPUSH', 'orchestrator:' .. orchestrator .. ':slots.reclaimed', slot)
    redis.call('SMOVE', 'sessions.active', 'sessions.terminated', sessionID)
    redis.call('HSET', 'session:'  .. sessionID .. ':status', 'terminatedAt', currentTime)
    redis.call('EXPIRE', 'session:' .. sessionID .. ':heartbeat.node', 1)
    "
    .to_string()
}

/// Lua script to extract the orchestrator from a session
pub fn fetch_orchestrator_from_session() -> String {
    // Variables that have to be defined:
    // sessionID
    // Variables that are being defined:
    // orchestrator
    r"local orchestrator = redis.call('rpoplpush', 'session:' .. sessionID .. ':orchestrator', 'session:' .. sessionID .. ':orchestrator')".to_string()
}
