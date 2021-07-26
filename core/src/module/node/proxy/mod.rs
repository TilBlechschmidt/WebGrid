use async_trait::async_trait;
use hyper::Server;
use jatsl::Job;
use std::net::SocketAddr;

use self::forwarding::ForwardingResponder;
use self::terminate::TerminationInterceptor;
use crate::harness::HeartStone;
use crate::make_responder_chain_service_fn;

mod forwarding;
mod terminate;

pub struct ProxyJob {
    port: u16,
    identifier: String,
    authority: String,
    session_id_internal: String,
    session_id_external: String,
    heart_stone: HeartStone,
}

impl ProxyJob {
    pub fn new(
        port: u16,
        identifier: String,
        authority: String,
        session_id_internal: String,
        session_id_external: String,
        heart_stone: HeartStone,
    ) -> Self {
        Self {
            port,
            identifier,
            authority,
            session_id_internal,
            session_id_external,
            heart_stone,
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

        let forwarding_responder = ForwardingResponder::new(
            self.identifier.clone(),
            self.authority.clone(),
            self.session_id_internal.clone(),
            self.session_id_external.clone(),
        );

        let make_svc = make_responder_chain_service_fn! {
            termination_interceptor,
            forwarding_responder
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let server = Server::try_bind(&addr)?.http2_only(true).serve(make_svc);
        let graceful = server.with_graceful_shutdown(manager.termination_signal());

        manager.ready().await;
        graceful.await?;

        Ok(())
    }
}
