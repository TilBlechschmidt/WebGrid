//! Various options usable by modules
//!
//! The structs in this module allow other modules to flatten them into
//! their own options struct. This allows for a unified yet non-cluttered
//! option set.

use domain::SessionMetadata;
use library::helpers::parse_seconds;
use library::storage::{parse_storage_backend_uri, s3::S3StorageBackend};
use mongodb::error::ErrorKind;
use mongodb::options::{CreateCollectionOptions, CreateIndexOptions, IndexOptions};
use mongodb::{Client, Collection, Database, IndexModel};
use std::time::Duration;
use structopt::StructOpt;
use tracing::{trace, warn};

/// Options for connecting to the Redis server
#[derive(Debug, StructOpt)]
pub struct RedisOptions {
    /// Redis database server URL
    #[structopt(
        short = "r",
        long = "redis",
        env = "REDIS",
        global = true,
        default_value = "redis://webgrid-redis/",
        value_name = "url"
    )]
    pub url: String,
}

/// Options relevant for message queueing
#[derive(Debug, StructOpt)]
pub struct QueueingOptions {
    /// Unique and stable identifier for this instance.
    /// It is used to identify and resume work after a crash
    /// or deliberate restart, thus it may not change across
    /// executions!
    #[structopt(env)]
    pub id: String,
}

/// Options for connection to a storage backend
#[derive(Debug, StructOpt)]
pub struct StorageOptions {
    /// Storage backend URL
    #[structopt(name = "storage", long, env = "STORAGE", parse(try_from_str = parse_storage_backend_uri))]
    pub backend: Option<S3StorageBackend>,
}

/// Options regarding the permanent storage backend
#[derive(Debug, StructOpt)]
pub struct MongoDBOptions {
    /// MongoDB connection URL
    #[structopt(long, env)]
    mongodb: String,

    /// Name of the database to use
    #[structopt(long, env, default_value = "webgrid")]
    database: String,

    /// Name of the capped collection where session metadata is archived
    #[structopt(long, env, default_value = "sessions")]
    collection: String,

    /// Name of the collection where session metadata is aggregated
    #[structopt(long, env, default_value = "sessionsStaging")]
    staging_collection: String,

    /// Maximum duration a session metadata object may spend in the staging area.
    /// If, for some reason (e.g. orphaned), it should spend longer in there it will get garbage collected.
    #[structopt(long, env, default_value = "3600", parse(try_from_str = parse_seconds))]
    staging_ttl: Duration,

    /// Uncompressed size limit for the database collection.
    /// Defaults to 16GB uncompressed which should comfortably fit into a 16GB volume.
    /// Note that once a collection has been created, changing this value does nothing!
    #[structopt(long, env, default_value = "17179869184")]
    pub size_limit: u64,
}

impl MongoDBOptions {
    /// Instantiates a new database client instance
    pub async fn client(&self) -> mongodb::error::Result<Client> {
        Client::with_uri_str(&self.mongodb).await
    }

    /// Instantiates a new database connection based on a new client
    pub async fn database(&self) -> mongodb::error::Result<Database> {
        Ok(self.client().await?.database(&self.database))
    }

    /// Creates a new handle to the main collection
    pub async fn collection(
        &self,
        database: &Database,
    ) -> mongodb::error::Result<Collection<SessionMetadata>> {
        let options = CreateCollectionOptions::builder()
            .capped(true)
            .size(self.size_limit)
            .build();

        let upsert_collection = async {
            trace!("Attempting to create main collection");
            if let Err(e) = database.create_collection(&self.collection, options).await {
                if let ErrorKind::Command(ce) = (*e.kind).clone() {
                    if ce.code == 48 {
                        trace!("Main collection already exists");
                        return Ok(());
                    }
                }

                warn!(error = ?e, "Failed to create main collection");
                return Err(e);
            }

            Ok(())
        };
        upsert_collection.await?;
        Ok(database.collection(&self.collection))
    }

    /// Creates a new handle to the staging collection
    pub async fn staging_collection(
        &self,
        database: &Database,
    ) -> mongodb::error::Result<Collection<SessionMetadata>> {
        let upsert_collection = async {
            trace!("Attempting to create staging collection");
            if let Err(e) = database
                .create_collection(&self.staging_collection, CreateCollectionOptions::default())
                .await
            {
                if let ErrorKind::Command(ce) = (*e.kind).clone() {
                    if ce.code == 48 {
                        trace!("Staging collection already exists");
                        return Ok(());
                    }
                }

                warn!(error = ?e, "Failed to create staging collection");
                return Err(e);
            }

            Ok(())
        };
        upsert_collection.await?;

        let collection = database.collection(&self.staging_collection);

        let index_model = IndexModel::builder()
            .keys(mongodb::bson::doc! { "createdAt": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(self.staging_ttl)
                    .build(),
            )
            .build();

        trace!("Ensuring that staging TTL index exists");
        collection
            .create_index(index_model, CreateIndexOptions::default())
            .await?;

        Ok(collection)
    }
}
