import Redis from 'ioredis'
import { ApolloServer } from 'apollo-server'
import { resolvers } from './resolvers'
import { v4 as uuidv4 } from 'uuid'
import typeDefs from './schema'

async function heartbeat(redis, apiID, host) {
    redis.setex(`api:${apiID}:host`, 60, host)
}

async function start(redis, apiID, host) {
    const server = new ApolloServer({
        typeDefs,
        resolvers,
        context: { redis },
        playground: true,
        introspection: true,
        cors: true
    })

    await heartbeat(redis, apiID, host)
    setInterval(() => heartbeat(redis, apiID, host), 30 * 1000)

    const info = await server.listen()
    console.log(`🚀  Server ready at ${info.url}`)
}

const { program } = require('commander')

program
    .requiredOption('-r, --redis <url>', 'Redis database server URL (env: REDIS=)', process.env.WEBGRID_REDIS_URL || 'redis://webgrid-redis/')
    .requiredOption('--host <hostname>', 'Host under which the manager is reachable by other services (env: HOST=)', process.env.HOST)

program.parse(process.argv)

const redis = new Redis(program.redis)
const apiID = uuidv4()

start(redis, apiID, program.host).then(() => { }).catch(err => {
    console.error('Server threw error:', err)
})

process.on('SIGINT', () =>
    redis.expire(`api:${apiID}:host`, 1).then(process.exit).catch(err => {
        console.error('Error deregistering', err)
        process.exit()
    })
)