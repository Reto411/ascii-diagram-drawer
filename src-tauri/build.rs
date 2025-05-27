use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=../shared/messages/*");
    prost_build::Config::new()
        .compile_protos(&collect_proto_files(), &["../shared/messages"])
        .unwrap();
    tauri_build::build()
}

fn collect_proto_files() -> Vec<std::path::PathBuf> {
    let proto_dir = "../shared/messages";
    // Collect all .proto files in the directory
    let protos: Vec<_> = fs::read_dir(proto_dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.extension().and_then(|s| s.to_str()) == Some("proto") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    protos
}