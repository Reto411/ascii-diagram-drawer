use std::io::Error;
use std::path::{Path, PathBuf};
use log::info;
use crate::shape::ShapeCollection;

pub mod drawable;
mod shape;

// Constants
const RESOURCE_PATH : &str = "./resources/";


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn route_load_all_shape_collections() -> Result<Vec<shape::ShapeCollection>, String> {
    let result = shape::load_all_shape_collections().await;
    match result {
        Ok(_) => { Ok(result.unwrap()) }
        Err(_) => { Err("Error loading shape collections".to_string()) }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![route_load_all_shape_collections, greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
