use std::{
    env,
    fs,
    path::PathBuf,
    process::Command,
};

fn main() {
    // Proto input dir
    let proto_dir = PathBuf::from("proto");

    // Main proto file
    let proto_file_path = proto_dir.join("api.proto");

    // Where to put prost .rs output
    let out_proto_dir = PathBuf::from("src/generated");

    fs::create_dir_all(&out_proto_dir).expect("Failed to create src/generated");

    // OUT_DIR for descriptor set
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_set_path = out_dir.join("api.desc");

    // Compile with prost-build
    prost_build::Config::new()
        .out_dir(&out_proto_dir)
        .compile_protos(&[proto_file_path.as_path()], &[proto_dir.as_path()])
        .expect("Failed to compile protos");

    // Also run protoc to generate descriptor set
    let status = Command::new("protoc")
        .args([
            "--proto_path=proto",
            "--include_imports",
            "--include_source_info",
            "--descriptor_set_out",
            descriptor_set_path.to_str().unwrap(),
        ])
        .arg("proto/api.proto")
        .arg("proto/api_options.proto")
        .status()
        .expect("Failed to run protoc");

    assert!(status.success(), "protoc failed");

    // Tell cargo when to rerun
    println!("cargo:rerun-if-changed=proto/api.proto");
    println!("cargo:rerun-if-changed=proto/api_options.proto");
}
