export default {
    alive: ({ id }, _, { redis }) => redis.get(`session:${id}:heartbeat.node`).then(res => Boolean(res)),

    slot: ({ id }, _, { redis }) => redis.get(`session:${id}:slot`),
    orchestrator: ({ id }, _, { redis }) => redis.rpoplpush(`session:${id}:orchestrator`, `session:${id}:orchestrator`),

    status: ({ id }, _, { redis }) => redis.hgetall(`session:${id}:status`),
    capabilities: ({ id }, _, { redis }) => redis.hgetall(`session:${id}:capabilities`),

    upstream: ({ id }, _, { redis }) => redis.hgetall(`session:${id}:upstream`),
    downstream: ({ id }, _, { redis }) => redis.hgetall(`session:${id}:downstream`),

    storage: ({ id }, _, { redis }) => redis.get(`session:${id}:storage`),
    videoURL: async ({ id }, _, { redis }) => {
        const storageID = await redis.get(`session:${id}:storage`)
        return `/storage/${storageID}/${id}.m3u8`
    }
}