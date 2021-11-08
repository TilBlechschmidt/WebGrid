use crate::gangway::proxy::api::ApiForwardingResponder;
use crate::gangway::proxy::storage::StorageResponder;
use async_trait::async_trait;
use domain::WebgridServiceDescriptor;
use hyper::Server;
use jatsl::Job;
use library::communication::discovery::ServiceDiscoverer;
use library::storage::StorageBackend;
use library::{http::Responder, make_responder_chain_service_fn, responder_chain};
use std::net::SocketAddr;
use tracing::info;

use self::{create::SessionCreationResponder, session::SessionForwardingResponder};
use super::SessionCreationCommunicationHandle;

mod api;
mod create;
mod error;
mod session;
mod storage;

pub struct ProxyJob<D: ServiceDiscoverer<WebgridServiceDescriptor>, S: StorageBackend> {
    port: u16,
    identifier: String,
    discoverer: D,
    handle: SessionCreationCommunicationHandle,
    storage: Option<S>,
}

impl<D: ServiceDiscoverer<WebgridServiceDescriptor>, S: StorageBackend> ProxyJob<D, S> {
    pub fn new(
        port: u16,
        identifier: String,
        discoverer: D,
        handle: SessionCreationCommunicationHandle,
        storage: Option<S>,
    ) -> Self {
        Self {
            port,
            identifier,
            discoverer,
            handle,
            storage,
        }
    }
}

#[async_trait]
impl<D, S> Job for ProxyJob<D, S>
where
    S: StorageBackend + Send + Sync + Clone + 'static,
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
            SessionForwardingResponder::new(self.identifier.clone(), discoverer.clone());
        let creation_responder = SessionCreationResponder::new(self.handle.clone());
        let storage_responder = StorageResponder::new(self.storage.clone());
        let api_responder = ApiForwardingResponder::new(self.identifier.clone(), discoverer);

        let make_svc = make_responder_chain_service_fn! {
            session_responder,
            creation_responder,
            storage_responder,
            api_responder
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let server = Server::try_bind(&addr)?.serve(make_svc);
        let graceful = server.with_graceful_shutdown(manager.termination_signal());

        info!(?addr, "Serving WebGrid");
        manager.ready().await;
        graceful.await?;

        Ok(())
    }
}
