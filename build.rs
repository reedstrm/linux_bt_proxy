use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_dir = manifest_dir.join("proto");

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(
            &[proto_dir.join("api.proto"), proto_dir.join("api_options.proto")],
            &[proto_dir],
        )
        .expect("Failed to compile .proto files");

    // Rename undesired _.rs to api.rs
    let src = out_dir.join("_.rs");
    let dst = out_dir.join("api.rs");
    if src.exists() {
        fs::rename(src, dst).expect("Failed to rename _.rs to api.rs");
    }
}
