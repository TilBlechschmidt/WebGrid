use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use serde::Deserialize;
use sqlx::{Connection, SqliteConnection};
use std::env;
use std::fs::{read_to_string, remove_file, File};
use std::io::prelude::*;
use std::path::PathBuf;
use std::{collections::HashMap, str::FromStr};
// use vergen::{vergen, Config, SemverKind, ShaKind};

#[derive(Debug, Deserialize)]
struct Constants {
    ports: Ports,
}

#[derive(Debug, Deserialize)]
struct Ports {
    base: u16,
    offsets: HashMap<String, u16>,
}

#[tokio::main]
async fn main() {
    generate_typecheck_database().await;
    generate_constants();
    generate_version_info();
}

fn generate_version_info() {
    let repo_dir = if let Some(path_str) = option_env!("WEBGRID_GIT_REPOSITORY") {
        PathBuf::from_str(path_str).unwrap()
    } else {
        let rust_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
        rust_dir.parent().unwrap().to_path_buf()
    };

    let repository = Repository::open(&repo_dir).unwrap();

    let mut describe_opts = DescribeOptions::new();
    describe_opts.describe_tags();

    let mut describe_format_opts = DescribeFormatOptions::new();
    describe_format_opts.dirty_suffix("-dirty");

    let description = repository.describe(&describe_opts).unwrap();
    let version = description.format(Some(&describe_format_opts)).unwrap();

    println!("cargo:rustc-env=WEBGRID_VERSION={}", version);
    println!("cargo:rerun-if-changed={}", repo_dir.join(".git").display());
}

async fn generate_typecheck_database() {
    let in_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Get a path to the database
    let database_path = out_dir.join("build.db");

    // Remove any previous versions of the database
    remove_file(&database_path).ok();

    // Get a path to the schema file
    let schema_file_path = in_dir.join("src/libraries/storage/sql/schema.sql");

    // Create a database (for some reason we have to create an empty file to make SQLite happy 🤷‍♂️)
    File::create(&database_path).unwrap();
    let database_url = format!("sqlite:{}", database_path.display());
    let mut con = SqliteConnection::connect(&database_url).await.unwrap();

    // Import the schema
    let schema = read_to_string(schema_file_path).unwrap();
    sqlx::query(&schema).execute(&mut con).await.unwrap();

    // Close the database
    con.close().await.unwrap();

    // Set the DATABASE_URL and if-changed values
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/libraries/storage/sql/schema.sql");
    println!(
        "cargo:rustc-env=DATABASE_URL=sqlite://{}",
        database_path.display()
    );
}

fn generate_constants() {
    let constants_definition_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("./constants.yml");

    let constants_definition_file = File::open(constants_definition_path).unwrap();
    let constants: Constants = serde_yaml::from_reader(constants_definition_file).unwrap();

    let constants_output_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("constants.rs");
    let mut constants_output_file = File::create(constants_output_path).unwrap();

    for (service, offset) in constants.ports.offsets {
        let service_uppercase = service.to_uppercase();
        let port = constants.ports.base + offset;

        // Write port constant as &str
        constants_output_file
            .write_fmt(format_args!(
                "pub const PORT_{}: &str = \"{}\";\n",
                service_uppercase, port
            ))
            .unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=constants.yml");
    println!("cargo:rustc-env=ENV_PREFIX=WEBGRID_");
}
