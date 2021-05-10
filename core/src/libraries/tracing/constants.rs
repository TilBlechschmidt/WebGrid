pub mod service {
    pub const NAMESPACE: &str = "Webgrid";

    pub const PROXY: &str = "Proxy";
    pub const MANAGER: &str = "Manager";
    pub const ORCHESTRATOR: &str = "Orchestrator";
    pub const STORAGE: &str = "Storage";
    pub const NODE: &str = "Node";
    pub const API: &str = "API";
}

pub mod trace {
    use opentelemetry::Key;

    pub const NET_UPSTREAM_NAME: Key = Key::from_static_str("net.upstream.name");

    pub const SESSION_ID: Key = Key::from_static_str("webgrid.session.id");
    pub const SESSION_ORCHESTRATOR: Key = Key::from_static_str("webgrid.session.orchestrator");
    pub const SESSION_HOST: Key = Key::from_static_str("webgrid.session.host");
    pub const SESSION_CONTAINER_IMAGE: Key =
        Key::from_static_str("webgrid.session.container.image");
    pub const SESSION_CONTAINER_NAME: Key = Key::from_static_str("webgrid.session.container.name");
    pub const SESSION_ID_INTERNAL: Key = Key::from_static_str("webgrid.session.id.internal");
}
