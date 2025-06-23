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

    // Process descriptor
    process_descriptor(&descriptor_set_path, &out_proto_dir);

    // Tell cargo when to rerun
    println!("cargo:rerun-if-changed=proto/api.proto");
    println!("cargo:rerun-if-changed=proto/api_options.proto");
}

fn process_descriptor(desc_path: &PathBuf, out_proto_dir: &PathBuf) {
    use prost::Message;

    let bytes = std::fs::read(desc_path).expect("Failed to read descriptor");

    let desc: prost_types::FileDescriptorSet =
        prost_types::FileDescriptorSet::decode(&*bytes).expect("Failed to parse descriptor");

    let mut out = String::new();
    out.push_str("// AUTO-GENERATED, DO NOT EDIT\n");
    out.push_str("pub static MESSAGE_TYPE_MAP: &[(&str, u32)] = &[\n");

    for file in &desc.file {
        for message in &file.message_type {
            if let Some(options) = &message.options {
                for unknown in &options.uninterpreted_option {
                    for part in &unknown.name {
                        if part.name_part == "id" {
                            if let Some(id_value) = unknown.positive_int_value {
                                out.push_str(&format!(
                                    "    (\"{}\", {}),\n",
                                    message.name.as_ref().unwrap(),
                                    id_value
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    out.push_str("];\n");

    let out_path = out_proto_dir.join("message_ids.rs");
    std::fs::write(&out_path, out).expect("Failed to write message_ids.rs");

    println!(
        "cargo:warning=Generated message_ids.rs at {}",
        out_path.display()
    );
}
