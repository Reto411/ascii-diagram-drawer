use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    println!("cargo:rerun-if-changed=../shared/messages/**/*");
    // Recursively collect all .proto files in the directory and subdirectories
    let proto_dir = "../shared/messages";
    let protos: Vec<PathBuf> = WalkDir::new(proto_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("proto"))
        .map(|entry| entry.path().to_path_buf())
        .collect();
    prost_build::Config::new()
        .compile_protos(&protos, &[proto_dir])
        .unwrap();
    tauri_build::build()
}