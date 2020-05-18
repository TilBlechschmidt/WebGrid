use std::net::SocketAddr;

static BASE_PORT: u16 = 40001;

pub enum ServicePort {
    Manager = 0,
    Metrics = 1,
    Node = 2,
    Orchestrator = 3,
    Proxy = 4,
}

impl ServicePort {
    pub fn port(self) -> u16 {
        BASE_PORT + (self as u16)
    }

    pub fn socket_addr(self) -> SocketAddr {
        ([0, 0, 0, 0], self.port()).into()
    }
}
