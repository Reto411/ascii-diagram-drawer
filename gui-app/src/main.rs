//! ascii-diagram-drawer GUI entry point (ADR 008, S-019): a thin binary that
//! delegates to the library target.
//!
//! - **Native:** [`gui_app::run_native`] — the eframe desktop app. CLI arguments: an
//!   optional `.ad` file to load at startup and `--collections <dir>` for user shape
//!   collections (see the crate docs).
//! - **Wasm:** no-op — the browser drives the app through the `#[wasm_bindgen]`
//!   [`gui_app::web::WebHandle`] glue in the library target (Trunk builds the lib,
//!   not this binary).

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    gui_app::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
