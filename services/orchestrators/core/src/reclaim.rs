use redis::{aio::MultiplexedConnection, RedisResult};
use chrono::Utc;

pub async fn reclaim_slots(
    con: &mut MultiplexedConnection,
    orchestrator_id: &str,
) -> RedisResult<(Vec<String>, Vec<String>)> {
    // TODO: The terminate_session function is a duplicate of shared::lifecycle, extract it! Maybe a lua builder or smth similar in shared library?
    // Arg 1: Orchestrator ID
    // Arg 2: Current timestamp
    let script = redis::Script::new(
        r"
        local function has_value(tab, val)
            for index, value in ipairs(tab) do
                if value == val then
                    return true
                end
            end

            return false
        end

        local function terminate_session(sessionID, orchestrator, currentTime)
            local slot = redis.pcall('GET', 'session:' .. sessionID .. ':slot')
            redis.pcall('DEL', 'session:' .. sessionID .. ':slot')
            redis.pcall('RPUSH', 'orchestrator:' .. orchestrator .. ':slots.reclaimed', slot)
            redis.pcall('SMOVE', 'sessions.active', 'sessions.terminated', sessionID)
            redis.pcall('HSET', 'session:'  .. sessionID .. ':status', 'terminatedAt', currentTime)
            redis.pcall('DEL', 'session:' .. sessionID .. ':heartbeat.node')
            return slot
        end

        local function reclaim_slots(orchestratorID, currentTime)
            local reclaimed = redis.call('LRANGE', 'orchestrator:' .. orchestratorID .. ':slots.reclaimed', 0, -1)
            local inUse = redis.call('LRANGE', 'orchestrator:' .. orchestratorID .. ':slots.available', 0, -1)
            
            local ownSlots = redis.call('SMEMBERS', 'orchestrator:' .. orchestratorID .. ':slots')
            local sessions = redis.call('SMEMBERS', 'sessions.active')
            
            -- Recover slots from dead sessions
            local dead = {}
            for k, sessionID in pairs(sessions) do
                local slot = redis.pcall('GET', 'session:' .. sessionID .. ':slot')
            
                if has_value(ownSlots, slot) then
                    local wasAlive = redis.call('HEXISTS', 'session:' .. sessionID .. ':status', 'aliveAt') == 1
                    local nodeAlive = redis.call('EXISTS', 'session:' .. sessionID .. ':heartbeat.node') == 1
                    local managerAlive = redis.call('EXISTS', 'session:' .. sessionID .. ':heartbeat.manager') == 1
            
                    local alive = ((not wasAlive) and managerAlive) or (wasAlive and nodeAlive)
            
                    if alive then
                        table.insert(inUse, slot)
                    else
                        table.insert(dead, slot)
                        table.insert(reclaimed, slot)
                        terminate_session(sessionID, orchestratorID, currentTime)
                    end
                end
            end
            
            -- Recover orphaned slots
            local orphaned = {}
            for k, slot in pairs(ownSlots) do
                if not (has_value(reclaimed, slot) or has_value(inUse, slot)) then
                    table.insert(orphaned, slot)
                    table.insert(reclaimed, slot)
                    redis.call('RPUSH', 'orchestrator:' .. orchestratorID .. ':slots.reclaimed', slot)
                end
            end
            
            return {dead, orphaned}
        end

        return reclaim_slots(ARGV[1], ARGV[2])
    ",
    );

    let res: RedisResult<Vec<Vec<String>>> = script
        .arg(orchestrator_id)
        .arg(Utc::now().to_rfc3339())
        .invoke_async(con)
        .await;

    match res {
        Ok(mut result) => {
            let dead_slots = result.remove(0);
            let orphaned_slots = result.remove(0);

            Ok((dead_slots, orphaned_slots))
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::{aio::MultiplexedConnection, cmd, Client, RedisResult, Script};

    async fn setup_test() -> MultiplexedConnection {
        let redis_url = "redis://localhost/";
        let client = Client::open(redis_url).unwrap();
        let mut con = client.get_multiplexed_tokio_connection().await.unwrap();

        let _: RedisResult<()> = cmd("FLUSHALL").query_async(&mut con).await;

        return con;
    }

    async fn cleanup(con: &mut MultiplexedConnection) {
        let _: RedisResult<()> = cmd("FLUSHALL").query_async(con).await;
    }

    async fn load_test_data(con: &mut MultiplexedConnection) {
        // Creates the following:
        // 5 Sessions
        // -> session1 is assigned to a manager which is alive
        // -> session2 is assigned to a node which is alive
        // -> session3 is assigned to a manager which is dead
        // -> session4 is assigned to a node which is dead
        // -> session5 belongs to a different orchestrator
        // 7 Slots
        // -> slot1 bound to session1
        // -> slot2 bound to session2
        // -> slot3 bound to session3
        // -> slot4 bound to session4
        // -> slot5 is available
        // -> slot6 is reclaimed
        // -> slot7 is orphaned (not bound to anything nor available)

        let script = Script::new(
            r#"
            redis.call('SADD', 'sessions.active', 'session1', 'session2', 'session3', 'session4', 'session5')

            -- Slots 1-6 are assigned or unused, slot 7 is "gone"
            redis.call('SADD', 'orchestrator:test:slots', 'slot1', 'slot2', 'slot3', 'slot4', 'slot5', 'slot6', 'slot7')
            redis.call('LPUSH', 'orchestrator:test:slots.available', 'slot5')
            redis.call('LPUSH', 'orchestrator:test:slots.reclaimed', 'slot6')

            -- Session with alive manager
            redis.call('SET', 'session:session1:slot', 'slot1')
            redis.call('SET', 'session:session1:heartbeat.manager', '42')

            -- Session with alive node
            redis.call('SET', 'session:session2:slot', 'slot2')
            redis.call('SET', 'session:session2:heartbeat.node', '42')
            redis.call('HSET', 'session:session2:status', 'aliveAt', '42')

            -- Session thats dead
            redis.call('SET', 'session:session3:slot', 'slot3')

            -- Session that has been alive but did not return the slot
            redis.call('SET', 'session:session4:slot', 'slot4')
            redis.call('HSET', 'session:session4:status', 'aliveAt', '42')

            -- Session that belongs to someone else
            redis.call('SET', 'session:session5:slot', 'somebodyelsesslot')
        "#,
        );

        let _: Option<()> = script.prepare_invoke().invoke_async(con).await.ok();
    }

    #[tokio::test]
    async fn online_evaluation() {
        let mut con = setup_test().await;

        load_test_data(&mut con).await;

        let (dead, orphaned) = reclaim_slots(&mut con, &"test").await.unwrap();

        assert_eq!(dead.len(), 2);
        assert!(dead.contains(&"slot3".to_string()));
        assert!(dead.contains(&"slot4".to_string()));
        assert_eq!(orphaned, ["slot7"]);

        cleanup(&mut con).await;
    }
}
