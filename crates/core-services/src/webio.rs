//! Web IO (S-019, ADR 009): the browser is the only filesystem a web build has.
//!
//! [`download_bytes`] triggers a browser download (the save/export sink — the user's
//! OS stores it wherever its download flow puts it), and [`WebStore`] keeps session
//! documents in memory while persisting them as downloads on `save`. Platform DOM
//! access is confined to this module, wasm32-only (C3).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use diagram_model::Diagram;
use ui_api::{DiagramStore, LoadedDocument, StoreError};
use wasm_bindgen::JsCast;

/// Triggers a browser download of `bytes` as `filename` with MIME type `mime`
/// (Blob → object URL → synthetic `<a download>` click). The object URL is not
/// revoked: the browser fetches it asynchronously after the click, and revoking
/// immediately would race that fetch on some engines — one small URL per download
/// is reclaimed when the page unloads.
///
/// # Errors
/// A message when the DOM is unavailable or the anchor cannot be created — the GUI
/// surfaces it in its status bar like any other IO failure.
pub fn download_bytes(filename: &str, mime: &str, bytes: Vec<u8>) -> Result<(), String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let document = window.document().ok_or("no document")?;

    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    // The Blob parts are a JS array with one binary-safe part (PNG survives as-is).
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));
    let blob = web_sys::Blob::new_with_u8_slice_sequence_and_options(&parts, &options)
        .map_err(|e| format!("could not create the download blob: {e:?}"))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("could not create an object URL: {e:?}"))?;

    let anchor = document
        .create_element("a")
        .map_err(|e| format!("could not create the download link: {e:?}"))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|e| format!("bad download link element: {e:?}"))?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    document
        .body()
        .ok_or("no document body")?
        .append_child(&anchor)
        .map_err(|e| format!("could not attach the download link: {e:?}"))?;
    anchor.click();
    anchor.remove();
    Ok(())
}

/// In-memory [`DiagramStore`] for web builds (S-019): documents live for the session
/// and every `save` additionally triggers a browser download of the `.ad` text, so
/// nothing is lost when the tab closes. Same error vocabulary as the other stores.
#[derive(Debug, Default)]
pub struct WebStore {
    documents: Mutex<HashMap<PathBuf, Diagram>>,
}

impl DiagramStore for WebStore {
    fn load(&self, path: &Path) -> Result<LoadedDocument, StoreError> {
        // Session memory only — a reload of the page starts empty (by design on web;
        // the picker is the way back in). Unknown keys are IO errors like MemoryStore.
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
            .map(|diagram| LoadedDocument {
                skipped_unknowns: 0,
                diagram,
            })
            .ok_or_else(|| StoreError::Io {
                message: format!(
                    "no session document at {} (web build — open one via the picker)",
                    path.display()
                ),
            })
    }

    fn save(&self, path: &Path, diagram: &Diagram) -> Result<(), StoreError> {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.to_path_buf(), diagram.clone());
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("diagram.ad");
        download_bytes(
            filename,
            "text/plain",
            ad_format::serialize(diagram).into_bytes(),
        )
        .map_err(|message| StoreError::Io { message })
    }
}

// No unit tests here: `WebStore` only compiles for wasm32, where plain libtest
// binaries cannot execute (the wasip1 harness has no DOM for `download_bytes`, and
// the browser needs `#[wasm_bindgen_test]`). Its in-memory half is the trivial
// `MemoryStore` behavior already covered natively; the DOM download path is
// exercised by the gui-app web bundle smoke (S-019 AC1/AC4).
