use super::super::schema::{schema, GQLContext};
use super::super::Context;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::scheduling::{Job, TaskManager};
use crate::with_shared_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use warp::Filter;

#[derive(Clone)]
pub struct ServerJob {
    port: u16,
}

#[async_trait]
impl Job for ServerJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["OPTIONS", "GET", "POST"]);

        let redis = Arc::new(Mutex::new(with_shared_redis_resource!(manager)));
        let state = warp::any().map(move || GQLContext {
            redis: redis.clone(),
        });
        let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

        let graphql_route = warp::post().and(graphql_filter);
        let playground_route = warp::get().and(juniper_warp::playground_filter("/api", None));

        let routes = warp::path("api")
            .and(playground_route)
            .or(graphql_route)
            .with(cors);

        let source_addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let (addr, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(source_addr, manager.termination_signal());

        info!("Listening at {}/api", addr);
        manager.ready().await;

        server.await;

        Ok(())
    }
}

impl ServerJob {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}
