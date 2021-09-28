use crate::domain::SessionMetadata;
use crate::library::helpers::parse_seconds;
use crate::module::options::{QueueingOptions, RedisOptions};
use mongodb::error::ErrorKind;
use mongodb::options::{CreateCollectionOptions, CreateIndexOptions, IndexOptions};
use mongodb::{Client, Collection, Database, IndexModel};
use std::time::Duration;
use structopt::StructOpt;

/// Options for the manager module
#[derive(Debug, StructOpt)]
pub struct Options {
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub queueing: QueueingOptions,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub redis: RedisOptions,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub mongo: MongoDBOptions,
}

/// Options regarding the search engine to use
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
    pub async fn client(&self) -> mongodb::error::Result<Client> {
        Client::with_uri_str(&self.mongodb).await
    }

    pub async fn database(&self) -> mongodb::error::Result<Database> {
        Ok(self.client().await?.database(&self.database))
    }

    pub async fn collection(
        &self,
        database: &Database,
    ) -> mongodb::error::Result<Collection<SessionMetadata>> {
        let options = CreateCollectionOptions::builder()
            .capped(true)
            .size(self.size_limit)
            .build();

        let upsert_collection = async {
            if let Err(e) = database.create_collection(&self.collection, options).await {
                if let ErrorKind::Command(ce) = (*e.kind).clone() {
                    if ce.code == 48 {
                        return Ok(());
                    }
                }
                return Err(e);
            }

            Ok(())
        };
        upsert_collection.await?;
        Ok(database.collection(&self.collection))
    }

    pub async fn staging_collection(
        &self,
        database: &Database,
    ) -> mongodb::error::Result<Collection<SessionMetadata>> {
        let upsert_collection = async {
            if let Err(e) = database
                .create_collection(&self.staging_collection, CreateCollectionOptions::default())
                .await
            {
                if let ErrorKind::Command(ce) = (*e.kind).clone() {
                    if ce.code == 48 {
                        return Ok(());
                    }
                }
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

        collection
            .create_index(index_model, CreateIndexOptions::default())
            .await?;

        Ok(collection)
    }
}
