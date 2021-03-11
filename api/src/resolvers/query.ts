export default {
    orchestrators: (_, __, { redis }) => redis.smembers('orchestrators')
        .then(ids => ids.map(id => ({ id }))),


    sessions: async (_, { state, name, build }, { redis }) => {
        let sessionIDs = []

        if (state) {
            const key = state === 'Active' ? 'sessions.active' : 'sessions.terminated'
            sessionIDs = await redis.smembers(key)
        } else {
            const active = await redis.smembers('sessions.active')
            const terminated = await redis.smembers('sessions.terminated')
            sessionIDs = [...active, ...terminated]
        }

        // Filter by name/build if applicable
        if (name || build) {
            let filteredSessionIDs = []

            for (const sessionID of sessionIDs) {
                const metadata = (await redis.hgetall(`session:${sessionID}:metadata`)) || {}
                let nameMatches = !name || name == metadata.name
                let buildMatches = !build || build == metadata.build

                if (nameMatches && buildMatches) {
                    filteredSessionIDs.push(sessionID)
                }
            }

            sessionIDs = filteredSessionIDs
        }

        return sessionIDs.map(id => ({ id }))
    },

    session: (_, { id }) => ({ id }),
    orchestrator: (_, { id }) => ({ id }),

    timeouts: (_, __, { redis }) => redis.hgetall('timeouts')
}
