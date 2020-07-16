import { gql } from 'apollo-server-express'

export default gql`
enum SessionState {
    Active
    Terminated
}

type Query {
    sessions(state: SessionState): [Session!]!
    orchestrators: [Orchestrator!]!

    session(id: String!): Session
    orchestrator(id: String!): Orchestrator

    timeouts: Timeouts
}
`