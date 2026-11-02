//! Browser file access (S-019): the web stand-in for the native system dialogs.
//!
//! The DOM file picker is asynchronous (the read completes after a frame or two), so
//! [`pick_ad_file`] only *starts* it; the result is delivered through
//! [`take_picked`] on a later frame, where the app applies it like any other open
//! outcome. One hidden `<input type="file">` element is created once and reused for
//! every pick (no per-click DOM leak).

use std::sync::{Mutex, OnceLock};

use wasm_bindgen::JsCast;

/// A document chosen in the browser file picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedFile {
    /// The file name as shown by the OS (used as the document's display name).
    pub name: String,
    /// The full text content (`.ad` documents are plain text).
    pub text: String,
}

static PICKED: Mutex<Option<PickedFile>> = Mutex::new(None);
static INPUT: OnceLock<web_sys::HtmlInputElement> = OnceLock::new();

/// Creates the one shared hidden file input with its `change` handler attached.
fn picker_input() -> Result<web_sys::HtmlInputElement, String> {
    if let Some(input) = INPUT.get() {
        return Ok(input.clone());
    }
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no browser document")?;
    let input = document
        .create_element("input")
        .map_err(|e| format!("could not create the file input: {e:?}"))?
        .dyn_into::<web_sys::HtmlInputElement>()
        .map_err(|e| format!("bad file input element: {e:?}"))?;
    input
        .set_attribute("type", "file")
        .and_then(|()| input.set_attribute("accept", ".ad,text/plain"))
        .map_err(|e| format!("could not configure the file input: {e:?}"))?;
    let _ = input.style().set_property("display", "none");

    // The handler reads `input.files` on every change — one closure for the app's
    // whole lifetime, so it is intentionally leaked (`forget`) rather than dropped.
    let handler_input = input.clone();
    let on_change = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
        let Some(file) = handler_input.files().and_then(|files| files.item(0)) else {
            return;
        };
        let Ok(file) = file.dyn_into::<web_sys::File>() else {
            return;
        };
        let name = file.name();
        // `File.text()` (inherited from Blob) resolves to a string on the web event
        // loop; read it with `spawn_local` and deliver via `take_picked` on a later
        // frame. A failed read simply delivers nothing.
        let promise = file.text();
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(js) = promise.await else { return };
            let Some(text) = js.as_string() else { return };
            *PICKED
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some(PickedFile { name, text });
        });
    });
    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget();

    document
        .body()
        .ok_or("no document body")?
        .append_child(&input)
        .map_err(|e| format!("could not attach the file input: {e:?}"))?;
    INPUT.get_or_init(|| input.clone());
    Ok(input)
}

/// Opens the browser's file picker for `.ad` documents (the web twin of File ▾ Open).
/// The result arrives via [`take_picked`] on a later frame.
///
/// # Errors
/// A message when the DOM is unavailable — the app shows it in its status bar.
pub fn pick_ad_file() -> Result<(), String> {
    let input = picker_input()?;
    // Clear any previous selection so re-picking the same file still fires `change`.
    input.set_value("");
    *PICKED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    input.click();
    Ok(())
}

/// Takes the document picked by [`pick_ad_file`], if the async read has completed —
/// `None` while pending or when nothing was chosen. Call once per frame.
#[must_use]
pub fn take_picked() -> Option<PickedFile> {
    PICKED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
}
