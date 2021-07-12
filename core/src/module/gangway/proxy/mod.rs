use crate::domain::WebgridServiceDescriptor;
use crate::library::communication::discovery::ServiceDiscoverer;
use crate::make_responder_chain_service_fn;
use async_trait::async_trait;
use hyper::Server;
use jatsl::Job;
use std::net::SocketAddr;

use self::{create::SessionCreationResponder, session::SessionForwardingResponder};
use super::SessionCreationCommunicationHandle;

mod create;
mod session;

pub struct ProxyJob<D: ServiceDiscoverer<WebgridServiceDescriptor>> {
    port: u16,
    identifier: String,
    discoverer: D,
    handle: SessionCreationCommunicationHandle,
}

impl<D: ServiceDiscoverer<WebgridServiceDescriptor>> ProxyJob<D> {
    pub fn new(
        port: u16,
        identifier: String,
        discoverer: D,
        handle: SessionCreationCommunicationHandle,
    ) -> Self {
        Self {
            port,
            identifier,
            discoverer,
            handle,
        }
    }
}

#[async_trait]
impl<D> Job for ProxyJob<D>
where
    // TODO Potentially dangerous usage of 'static lifetime
    D: ServiceDiscoverer<WebgridServiceDescriptor> + Send + Sync + Clone + 'static,
    D::I: Send + Sync,
{
    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(
        &self,
        manager: jatsl::JobManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let discoverer = self.discoverer.clone();

        let session_responder =
            SessionForwardingResponder::new(self.identifier.clone(), discoverer);
        let creation_responder = SessionCreationResponder::new(self.handle.clone());

        let make_svc = make_responder_chain_service_fn! {
            session_responder,
            creation_responder
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let server = Server::bind(&addr).serve(make_svc);
        let graceful = server.with_graceful_shutdown(manager.termination_signal());

        manager.ready().await;
        graceful.await?;

        Ok(())
    }
}
