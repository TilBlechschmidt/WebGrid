// TODO Figure out if there is a reasonable way to use crate::keys

pub fn terminate_session() -> String {
    // Variables that have to be defined:
    // sessionID, orchestrator, currentTime
    // Variables that are being defined:
    // slot
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

pub fn fetch_orchestrator_from_session() -> String {
    // Variables that have to be defined:
    // sessionID
    // Variables that are being defined:
    // orchestrator
    r"local orchestrator = redis.call('rpoplpush', 'session:' .. sessionID .. ':orchestrator', 'session:' .. sessionID .. ':orchestrator')".to_string()
}
