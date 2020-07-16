use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

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
    let constants_definition_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../constants.yml");

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
    println!("cargo:rerun-if-changed=../../constants.yml");
    println!("cargo:rustc-env=ENV_PREFIX=WEBGRID_");
}
