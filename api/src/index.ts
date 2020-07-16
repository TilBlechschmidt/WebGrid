import Redis from 'ioredis'
import { ApolloServer } from 'apollo-server-express'
import { resolvers } from './resolvers'
import { v4 as uuidv4 } from 'uuid'
import typeDefs from './schema'
import express from 'express'
import cors from 'cors'
import { embed } from './generated'

async function heartbeat(redis, apiID, host, port) {
    redis.setex(`api:${apiID}:host`, 60, `${host}:${port}`)
}

async function start(redis, apiID, host, port) {
    const app = express()

    const server = new ApolloServer({
        typeDefs,
        resolvers,
        context: { redis },
        playground: true,
        introspection: true,
    })

    app.disable('x-powered-by')

    app.use(cors())

    app.get('/embed', (req, res) => {
        res.set('Content-Type', 'application/javascript')
        res.end(embed)
    })

    app.get('/embed/:sessionID', (req, res) => {
        const sessionID = req.params.sessionID
        res.end(`
            <html>
                <head>
                    <script>${embed}</script>
                </head>
                <body style="margin: 0">
                    <webgrid-video session-id="${sessionID}"></webgrid-video>
                </body>
            </html>
        `)
    })

    server.applyMiddleware({ app, path: '/api' })

    await heartbeat(redis, apiID, host, port)
    setInterval(() => heartbeat(redis, apiID, host, port), 30 * 1000)

    app.listen(port, () => console.log(`🚀  Server ready at http://localhost:${port}`))
}

const { program } = require('commander')

program
    .requiredOption('-r, --redis <url>', 'Redis database server URL (env: REDIS=)', process.env.REDIS || 'redis://webgrid-redis/')
    .requiredOption('--host <hostname>', 'Host under which the manager is reachable by other services (env: HOST=)', process.env.HOST)

program.parse(process.argv)

console.log(program)
console.log(program.redis)

const redis = new Redis(program.redis)
const apiID = uuidv4()

start(redis, apiID, program.host, 4000).then(() => { }).catch(err => {
    console.error('Server threw error:', err)
})

process.on('SIGINT', () =>
    redis.expire(`api:${apiID}:host`, 1).then(process.exit).catch(err => {
        console.error('Error deregistering', err)
        process.exit()
    })
)