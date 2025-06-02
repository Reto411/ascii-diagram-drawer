use crate::message_utlis::{clone_io_error_to_backend_error_occurred, clone_to_event_message_collection_loaded};
use crate::messages::events::BackendErrorOccurred;
use crate::session::Session;
use crate::shape_template_collections::ShapeCollection;
use log::info;
use prost::Message;
use std::any::{type_name, Any};
use std::fmt::format;
use std::io::Error;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
use svgbob::sauron::form;
use svgbob::sauron::web_sys::console::error;
use tauri::{AppHandle, Builder, Emitter, Listener, Manager};
use tokio::io::join;
use tokio::spawn;

pub mod messages {
    pub mod events {
        include!(concat!(env!("OUT_DIR"), "/ascii_diagram_drawer.events.rs"));
    }
    pub mod types {
        include!(concat!(env!("OUT_DIR"), "/ascii_diagram_drawer.types.rs"));
    }
}

pub mod drawable;
mod message_utlis;
mod session;
mod shape_template_collections;

// Constants
const RESOURCE_PATH: &str = "./resources/";

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn initialize(app: AppHandle, session: tauri::State<'_, Mutex<Session>>) -> Result<(), String> {
    let mut lock_session = session.lock().await;
    let res = lock_session.initialize();
    if (res.is_err()) {
        let error_message = format!(
            "Failed to initialize session: {}",
            res.err().unwrap().to_string()
        );
        let error_event = clone_io_error_to_backend_error_occurred(error_message);
        app.emit(error_event.0, error_event.1.encode_to_vec());
    }

    for mut shape_template_collection in &mut lock_session.loaded_shape_template_collections {
        let app_handle = app.clone();
        let res = shape_template_collection.get_shape_collection().await;
        match res {
            Ok(res) => {
                let collection = clone_to_event_message_collection_loaded(&res);
                let res = app.emit(collection.0, collection.1.encode_to_vec());
                if(res.is_err())
                {
                    panic!("Failed to emit ShapeCollectionLoaded event: {}", res.err().unwrap().to_string());
                }
            }
            Err(e) => {
                let error_message = format!(
                    "Failed to get error collection in path {}: {}",
                    shape_template_collection
                        .file_path
                        .to_str()
                        .unwrap_or("<invalid path>"),
                    e
                );
                let error_event = clone_io_error_to_backend_error_occurred(error_message);
                let res = app_handle.emit(error_event.0, error_event.1.encode_to_vec());
                if(res.is_err())
                {
                    panic!("Failed to emit BackendErrorOccurred event: {}", res.err().unwrap().to_string());
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn route_load_all_shape_collections() -> Result<Vec<u8>, String> {
    let result = shape_template_collections::load_all_shape_collections().await;
    let proto = message_utlis::to_result_route_load_all_shape_collections(result)?;
    Ok(proto.encode_to_vec())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            app.manage(Mutex::new(session::Session::new()));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![initialize])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
