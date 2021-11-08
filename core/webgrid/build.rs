use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use std::{env, path::PathBuf, str::FromStr};

fn main() {
    let repo_dir = if let Some(path_str) = option_env!("WEBGRID_GIT_REPOSITORY") {
        PathBuf::from_str(path_str).unwrap()
    } else {
        let rust_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
        rust_dir.parent().unwrap().parent().unwrap().to_path_buf()
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
