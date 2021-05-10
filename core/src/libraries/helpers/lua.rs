//! Lua functions used for Redis interaction

// TODO Figure out if there is a reasonable way to use crate::keys

/// Lua script to clean up a session object in the redis database
///
/// Returns the associated slot, moves it to the terminated state and removed the heartbeat.
/// The following variables have to be defined for the script to work:
/// - sessionID
/// - currentTime
///
/// If the following variable is set, the sessions slot will be returned to the given orchestrator:
/// - orchestrator
///
/// These variables are being defined by the script:
/// - slot
pub fn terminate_session() -> String {
    r"
    local slot = redis.call('GETDEL', 'session:' .. sessionID .. ':slot')
    
    if ( orchestrator and slot )
    then
        redis.call('RPUSH', 'orchestrator:' .. orchestrator .. ':slots.reclaimed', slot)
    end

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

/// Purges a session object and all related keys from the database
///
/// Takes the session ID as its first argument.
pub fn delete_session() -> String {
    r"
    local sessionID = ARGV[1];
    redis.call('SREM', 'sessions.active', sessionID)
    redis.call('SREM', 'sessions.terminated', sessionID)
    
    redis.call('DEL', 'session:' .. sessionID .. ':heartbeat.node')
    redis.call('DEL', 'session:' .. sessionID .. ':heartbeat.manager')
    
    redis.call('DEL', 'session:' .. sessionID .. ':slot')
    redis.call('DEL', 'session:' .. sessionID .. ':orchestrator')
    
    redis.call('DEL', 'session:' .. sessionID .. ':log')
    redis.call('DEL', 'session:' .. sessionID .. ':status')
    redis.call('DEL', 'session:' .. sessionID .. ':capabilities')
    
    redis.call('DEL', 'session:' .. sessionID .. ':upstream')
    redis.call('DEL', 'session:' .. sessionID .. ':downstream')
    
    redis.call('DEL', 'session:' .. sessionID .. ':storage')

    redis.call('DEL', 'session:' .. sessionID .. ':telemetry.creation')
    "
    .to_string()
}

/// Purges an orchestrator object and all related keys from the database
///
/// Takes the orchestrator ID as its first argument.
pub fn delete_orchestrator() -> String {
    r"
    local orchestratorID = ARGV[1];
    
    redis.call('SREM', 'orchestrators', orchestratorID)
    redis.call('DEL', 'orchestrator:' .. orchestratorID)
    
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':heartbeat')
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':retain')
    
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':capabilities:platformName')
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':capabilities:browsers')
    
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':slots.reclaimed')
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':slots.available')
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':slots')

    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':backlog')
    redis.call('DEL', 'orchestrator:' .. orchestratorID .. ':pending')
    "
    .to_string()
}
