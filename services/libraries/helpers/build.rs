fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-env=ENV_PREFIX=WEBGRID_");
}
