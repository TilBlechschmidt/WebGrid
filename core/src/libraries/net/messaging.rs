use serde::{Deserialize, Serialize};

use super::discovery::ServiceDiscoveryResponse;

#[derive(Serialize, Deserialize)]
pub enum Message {
    /// Request for endpoint details.
    ///
    /// The channel this is sent in determines which services respond.
    /// For example to discover a storage service you send this to `discovery.storage:${ID}`
    ServiceDiscoveryRequest,

    /// Response to a service discovery request
    ///
    /// Will always be sent on the `discovery` channel for semi-passive discovery.
    /// When one service sends a discovery request, others can passively cache the
    /// response by listening in to this channel.
    // TODO This is currently not sent wrapped in a message 🤷‍♂️
    ServiceDiscoveryResponse(ServiceDiscoveryResponse),
}
