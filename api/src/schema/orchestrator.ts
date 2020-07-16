import { gql } from 'apollo-server-express'

export default gql`
type Browser {
    name: String!
    version: String!
}

type OrchestratorCapabilities {
    _orchestratorID: String!

    platformName: String
    browsers: [Browser!]!
}

type OrchestratorSlots {
    _orchestratorID: String!

    allocated: [String!]!
    reclaimed: [String!]!
    available: [String!]!
}

type Orchestrator {
    id: String!

    type: String
    alive: Boolean!

    capabilities: OrchestratorCapabilities!
    slots: OrchestratorSlots!

    backlog: [String!]!
    pending: [String!]!
}
`