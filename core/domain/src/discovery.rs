use super::event::SessionIdentifier;
use library::communication::discovery::ServiceDescriptor;
use serde::{Deserialize, Serialize};

/// [`ServiceDescriptor`] implementation for discoverable WebGrid services
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebgridServiceDescriptor {
    /// [`Api`](crate::module::api) instance
    Api,
    /// [`Node`](crate::module::node) instance
    Node(SessionIdentifier),
}

impl ServiceDescriptor for WebgridServiceDescriptor {
    fn service_identifier(&self) -> String {
        match self {
            WebgridServiceDescriptor::Api => "api".into(),
            WebgridServiceDescriptor::Node(identifier) => format!("node-{}", identifier),
        }
    }
}
