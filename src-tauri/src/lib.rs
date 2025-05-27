use std::io::Error;
use std::path::{Path, PathBuf};
use log::info;
use prost::Message;
use crate::shape::ShapeCollection;

pub mod messages {
    pub mod events{
        include!(concat!(env!("OUT_DIR"), "/ascii_diagram_drawer.events.rs"));
    }
   pub mod types {
       include!(concat!(env!("OUT_DIR"), "/ascii_diagram_drawer.types.rs"));
   }
}

pub mod drawable;
mod shape;
mod message_utlis;

// Constants
const RESOURCE_PATH : &str = "./resources/";


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn route_load_all_shape_collections() -> Result<Vec<u8>, String> {
    let result = shape::load_all_shape_collections().await;
    let proto = message_utlis::to_result_route_load_all_shape_collections(result)?;
    Ok(proto.encode_to_vec())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![route_load_all_shape_collections, greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
