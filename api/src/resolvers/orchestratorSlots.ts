export default {
    allocated: ({ _orchestratorID }, _, { redis }) => redis.smembers(`orchestrator:${_orchestratorID}:slots`),
    available: ({ _orchestratorID }, _, { redis }) => redis.lrange(`orchestrator:${_orchestratorID}:slots.available`, 0, -1),
    reclaimed: ({ _orchestratorID }, _, { redis }) => redis.lrange(`orchestrator:${_orchestratorID}:slots.reclaimed`, 0, -1)
}