use super::event::SessionIdentifier;
use crate::library::communication::discovery::ServiceDescriptor;
use serde::{Deserialize, Serialize};

/// [`ServiceDescriptor`] implementation for discoverable WebGrid services
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebgridServiceDescriptor {
    /// [`Node`](crate::module::node::Node) instance
    Node(SessionIdentifier),
}

impl ServiceDescriptor for WebgridServiceDescriptor {
    fn service_identifier(&self) -> String {
        match self {
            WebgridServiceDescriptor::Node(identifier) => format!("node-{}", identifier),
        }
    }
}
