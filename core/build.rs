use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use serde::Deserialize;
use std::{collections::HashMap, env, fs::File, io::Write, path::PathBuf, str::FromStr};

#[derive(Debug, Deserialize)]
struct Constants {
    ports: Ports,
}

#[derive(Debug, Deserialize)]
struct Ports {
    base: u16,
    offsets: HashMap<String, u16>,
}

fn main() {
    generate_version_info();
    generate_constants();
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
                "pub const PORT_{}: u16 = {};\n",
                service_uppercase, port
            ))
            .unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=constants.yml");
}
