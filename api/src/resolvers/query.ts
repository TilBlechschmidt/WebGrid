export default {
    orchestrators: (_, __, { redis }) => redis.smembers('orchestrators')
        .then(ids => ids.map(id => ({ id }))),


    sessions: async (_, { state }, { redis }) => {
        if (state) {
            const key = state === 'Active' ? 'sessions.active' : 'sessions.terminated'
            return await redis.smembers(key).then(ids => ids.map(id => ({id})))
        } else {
            const active = await redis.smembers('sessions.active')
            const terminated = await redis.smembers('sessions.terminated')
            return [...active, ...terminated].map(id => ({id}))
        }
    },

    session: (_, { id }) => ({ id }),
    orchestrator: (_, { id }) => ({ id }),

    timeouts: (_, __, { redis }) => redis.hgetall('timeouts')
}
