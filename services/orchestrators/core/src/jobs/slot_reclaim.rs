use crate::Context;
use anyhow::Result;
use async_trait::async_trait;
use chrono::offset::Utc;
use helpers::{lua::terminate_session, Timeout};
use log::{error, info};
use redis::{aio::ConnectionLike, RedisResult, Script};
use resources::{with_shared_redis_resource, ResourceManager};
use scheduling::{Job, TaskManager};
use std::time::Duration;
use tokio::time;

#[derive(Clone)]
pub struct SlotReclaimJob {}

#[async_trait]
impl Job for SlotReclaimJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);
        let interval_seconds = Timeout::SlotReclaimInterval.get(&mut con).await as u64;
        let mut interval = time::interval(Duration::from_secs(interval_seconds));
        let orchestrator_id = manager.context.id.clone();

        manager.ready().await;

        loop {
            interval.tick().await;

            let result = self.reclaim_slots(&mut con, &orchestrator_id).await;
            if let Ok((dead, orphaned)) = result {
                info!("Reclaim cycle executed (D: {:?}, O: {:?})", dead, orphaned);
            } else {
                error!("Reclaim cycle failed with error {:?}", result);
            }
        }
    }
}

impl SlotReclaimJob {
    pub fn new() -> Self {
        Self {}
    }

    async fn reclaim_slots(
        &self,
        con: &mut impl ConnectionLike,
        orchestrator_id: &str,
    ) -> RedisResult<(Vec<String>, Vec<String>)> {
        // TODO: The terminate_session function is a duplicate of shared::lifecycle, extract it! Maybe a lua builder or smth similar in shared library?
        // Arg 1: Orchestrator ID
        // Arg 2: Current timestamp
        let script_content = format!(
            r#"
            local function has_value(tab, val)
                for index, value in ipairs(tab) do
                    if value == val then
                        return true
                    end
                end
    
                return false
            end
    
            local function terminate_session(sessionID, orchestrator, currentTime)
                {terminate_session}
                return slot
            end
    
            local function reclaim_slots(orchestratorID, currentTime)
                local reclaimed = redis.call('LRANGE', 'orchestrator:' .. orchestratorID .. ':slots.reclaimed', 0, -1)
                local inUse = redis.call('LRANGE', 'orchestrator:' .. orchestratorID .. ':slots.available', 0, -1)
                
                local ownSlots = redis.call('SMEMBERS', 'orchestrator:' .. orchestratorID .. ':slots')
                local sessions = redis.call('SMEMBERS', 'sessions.active')
                
                -- Recover slots from dead sessions
                local dead = {{}}
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
                local orphaned = {{}}
                for k, slot in pairs(ownSlots) do
                    if not (has_value(reclaimed, slot) or has_value(inUse, slot)) then
                        table.insert(orphaned, slot)
                        table.insert(reclaimed, slot)
                        redis.call('RPUSH', 'orchestrator:' .. orchestratorID .. ':slots.reclaimed', slot)
                    end
                end
                
                return {{dead, orphaned}}
            end
    
            return reclaim_slots(ARGV[1], ARGV[2])
        "#,
            terminate_session = terminate_session()
        );

        let script = Script::new(&script_content);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use scheduling::TaskResourceHandle;
    use testing::with_resource_manager;

    #[test]
    fn online_evaluation() {
        with_resource_manager!(manager, {
            let mut con = manager.redis(TaskResourceHandle::stub()).await.unwrap();

            load_test_data(&mut con).await;

            let job = SlotReclaimJob::new();
            let (dead, orphaned) = job.reclaim_slots(&mut con, &"test").await.unwrap();

            assert_eq!(dead.len(), 2);
            assert!(dead.contains(&"slot3".to_string()));
            assert!(dead.contains(&"slot4".to_string()));
            assert_eq!(orphaned, ["slot7"]);
        });
    }

    async fn load_test_data<C: ConnectionLike>(con: &mut C) {
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
}
