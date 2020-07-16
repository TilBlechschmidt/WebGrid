use sqlx::{Connect, Connection, SqliteConnection};
use std::env;
use std::fs::{read_to_string, remove_file};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let in_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Get a path to the database
    let database_path = out_dir.join("build.db");

    // Remove any previous versions of the database
    remove_file(&database_path).ok();

    // Get a path to the schema file
    let schema_file_path = in_dir.join("src/sql/schema.sql");

    // Create a database
    let database_url = format!("sqlite:{}", database_path.display());
    let mut con = SqliteConnection::connect(database_url).await.unwrap();

    // Import the schema
    let schema = read_to_string(schema_file_path).unwrap();
    sqlx::query(&schema).execute(&mut con).await.unwrap();

    // Close the database
    con.close().await.unwrap();

    // Set the DATABASE_URL and if-changed values
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/sql/schema.sql");
    println!(
        "cargo:rustc-env=DATABASE_URL=sqlite://{}",
        database_path.display()
    );
}
