use async_trait::async_trait;
use hyper::Server;
use jatsl::Job;
use std::net::SocketAddr;
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

use self::file_upload::FileUploadInterceptor;
use self::forwarding::ForwardingResponder;
use self::terminate::TerminationInterceptor;
use crate::node::proxy::metadata_extension::MetadataExtensionInterceptor;
use domain::event::SessionClientMetadata;
use harness::HeartStone;
use library::{http::Responder, make_responder_chain_service_fn, responder_chain};

mod file_upload;
mod forwarding;
mod metadata_extension;
mod terminate;

pub struct ProxyJob {
    port: u16,
    identifier: String,
    authority: String,
    session_id_internal: String,
    session_id_external: String,
    heart_stone: HeartStone,
    metadata_tx: UnboundedSender<SessionClientMetadata>,
}

impl ProxyJob {
    pub fn new(
        port: u16,
        identifier: String,
        authority: String,
        session_id_internal: String,
        session_id_external: String,
        heart_stone: HeartStone,
        metadata_tx: UnboundedSender<SessionClientMetadata>,
    ) -> Self {
        Self {
            port,
            identifier,
            authority,
            session_id_internal,
            session_id_external,
            heart_stone,
            metadata_tx,
        }
    }
}

#[async_trait]
impl Job for ProxyJob {
    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(
        &self,
        manager: jatsl::JobManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let termination_interceptor =
            TerminationInterceptor::new(self.heart_stone.clone(), self.session_id_external.clone());

        let metadata_extension_interceptor = MetadataExtensionInterceptor::new(
            self.metadata_tx.clone(),
            self.heart_stone.clone(),
            self.session_id_external.clone(),
        );

        let forwarding_responder = ForwardingResponder::new(
            self.identifier.clone(),
            self.authority.clone(),
            self.session_id_internal.clone(),
            self.session_id_external.clone(),
            self.heart_stone.clone(),
        );

        let file_upload_interceptor =
            FileUploadInterceptor::new(self.heart_stone.clone(), self.session_id_external.clone());

        let make_svc = make_responder_chain_service_fn! {
            termination_interceptor,
            metadata_extension_interceptor,
            file_upload_interceptor,
            forwarding_responder
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let server = Server::try_bind(&addr)?.http2_only(true).serve(make_svc);
        let graceful = server.with_graceful_shutdown(manager.termination_signal());

        info!(?addr, "Serving WebDriver API");
        manager.ready().await;
        graceful.await?;

        Ok(())
    }
}
