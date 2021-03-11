import { gql } from 'apollo-server-express'

export default gql`
scalar Date

type SessionStatusTransitions {
    queuedAt: Date
    pendingAt: Date
    aliveAt: Date
    terminatedAt: Date
}

type SessionCapabilities {
    requested: String
    actual: String
}

type SessionUpstream {
    host: String
    port: Int
    driverSessionID: String
}

type SessionDownstream {
    host: String
    userAgent: String
    lastSeen: Date
}

type SessionMetadata {
    name: String
    build: String
}

type Session {
    id: String!
    alive: Boolean

    slot: String
    orchestrator: String

    status: SessionStatusTransitions
    capabilities: SessionCapabilities
    metadata: SessionMetadata

    upstream: SessionUpstream
    downstream: SessionDownstream

    storage: String
    videoURL: String
}
`