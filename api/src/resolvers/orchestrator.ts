const idMapper = ({ id }) => ({ _orchestratorID: id })

export default {
    type: ({ id }, _, { redis }) => redis.hget(`orchestrator:${id}`, 'type'),
    alive: ({ id }, _, { redis }) => redis.get(`orchestrator:${id}:heartbeat`).then(res => Boolean(res)),

    capabilities: idMapper,
    slots: idMapper,

    backlog: ({ id }, _, { redis }) => redis.lrange(`orchestrator:${id}:backlog`, 0, -1),
    pending: ({ id }, _, { redis }) => redis.lrange(`orchestrator:${id}:pending`, 0, -1)
}
