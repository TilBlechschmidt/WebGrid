use super::super::schema::{schema, GqlContext};
use super::super::Context;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::with_shared_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use jatsl::{Job, TaskManager};
use log::info;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use warp::Filter;

pub struct ServerJob {
    port: u16,
}

#[async_trait]
impl Job for ServerJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        // Setup CORS
        let cors = warp::cors().allow_any_origin().allow_header("content-type");

        // Define GraphQL filters
        let redis = Arc::new(Mutex::new(with_shared_redis_resource!(manager)));
        let state = warp::any().map(move || GqlContext {
            redis: redis.clone(),
        });
        let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

        let graphql_route = warp::post()
            .and(graphql_filter)
            .with(cors.clone().allow_method("POST"));
        let playground_route = warp::get().and(juniper_warp::playground_filter("/api", None));
        let api_routes = warp::path("api").and(playground_route.or(graphql_route));

        // Define file serving routes
        let file_serving_path = &manager.context.web_root;
        let directory_route = warp::get().and(warp::fs::dir(file_serving_path.clone()));
        let embed_script_route = warp::any()
            .and(warp::path("embed"))
            .and(warp::path::end())
            .and(
                warp::get()
                    .and(warp::fs::file(file_serving_path.join("embed.js")))
                    .with(cors.clone().allow_method("GET")),
            );
        let embed_styles_route = warp::any()
            .and(warp::path("embed.css"))
            .and(warp::path::end())
            .and(
                warp::get()
                    .and(warp::fs::file(file_serving_path.join("embed.css")))
                    .with(cors.clone().allow_method("GET")),
            );
        // .with(cors.allow_method("GET"));
        let fallback_route = warp::get().and(warp::fs::file(file_serving_path.join("__app.html")));

        // Put everything together
        let routes = api_routes
            .or(embed_script_route)
            .or(embed_styles_route)
            .or(directory_route)
            .or(fallback_route);

        // Piece the server together
        let source_addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let (addr, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(source_addr, manager.termination_signal());

        info!("Serving files and API at {}", addr);
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
