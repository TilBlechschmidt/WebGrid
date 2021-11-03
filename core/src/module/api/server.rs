use async_trait::async_trait;
use jatsl::{Job, JobManager};
use mongodb::Collection;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;
use warp::Filter;

use super::schema::schema;
use crate::domain::SessionMetadata;
use crate::library::EmptyResult;
use crate::module::api::schema::GqlContext;

pub struct ServerJob {
    port: u16,
    web_root: PathBuf,

    storage_collection: Collection<SessionMetadata>,
    staging_collection: Collection<SessionMetadata>,
}

impl ServerJob {
    pub fn new(
        port: u16,
        web_root: PathBuf,
        storage_collection: Collection<SessionMetadata>,
        staging_collection: Collection<SessionMetadata>,
    ) -> Self {
        Self {
            port,
            web_root,
            storage_collection,
            staging_collection,
        }
    }
}

#[async_trait]
impl Job for ServerJob {
    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: JobManager) -> EmptyResult {
        // Setup CORS
        let cors = warp::cors().allow_any_origin().allow_header("content-type");

        // Define GraphQL filters
        let staging_collection = self.staging_collection.clone();
        let storage_collection = self.storage_collection.clone();
        let state = warp::any().map(move || GqlContext {
            staging_collection: staging_collection.clone(),
            storage_collection: storage_collection.clone(),
        });
        let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

        let graphql_route = warp::post()
            .and(graphql_filter)
            .with(cors.clone().allow_method("POST"))
            .with(warp::trace::named("gql"));
        let playground_route = warp::get()
            .and(juniper_warp::playground_filter("/api", None))
            .with(warp::trace::named("playground"));
        let api_routes = warp::path("api").and(playground_route.or(graphql_route));

        // Define file serving routes
        let file_serving_path = &self.web_root;
        let directory_route = warp::get()
            .and(warp::fs::dir(file_serving_path.clone()))
            .with(warp::trace::named("files_dir"));
        let embed_script_route = warp::any()
            .and(warp::path("embed"))
            .and(warp::path::end())
            .and(
                warp::get()
                    .and(warp::fs::file(file_serving_path.join("embed.js")))
                    .with(cors.clone().allow_method("GET")),
            )
            .with(warp::trace::named("embed_script"));
        let embed_styles_route = warp::any()
            .and(warp::path("embed.css"))
            .and(warp::path::end())
            .and(
                warp::get()
                    .and(warp::fs::file(file_serving_path.join("embed.css")))
                    .with(cors.clone().allow_method("GET")),
            )
            .with(warp::trace::named("embed_styles"));

        let fallback_route = warp::get()
            .and(warp::fs::file(file_serving_path.join("__app.html")))
            .with(warp::trace::named("fallback"));

        // Put everything together
        let routes = api_routes
            .or(embed_script_route)
            .or(embed_styles_route)
            .or(directory_route)
            .or(fallback_route)
            .with(warp::trace::request());

        // Piece the server together
        let source_addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let (addr, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(source_addr, manager.termination_signal());

        info!(?addr, "Serving files and API");
        manager.ready().await;
        server.await;

        Ok(())
    }
}
