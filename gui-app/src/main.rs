//! ascii-diagram-drawer GUI entry point (ADR 008): eframe/egui on native desktop and
//! browser wasm. The app itself lives in [`app`]; this is bootstrap only.
//!
//! Optional CLI argument: a `.ad` file to load at startup — used by the visual test
//! tiers (gui-testing skill) for deterministic golden captures.
//!
//! Wasm note (S-012): AC5 only requires that the app code *compiles* for
//! `wasm32-unknown-unknown`; the real web entry point (`eframe::WebRunner` + a
//! `#[wasm_bindgen]` glue struct, per eframe's documented pattern) lands in S-019.

mod app;
mod canvas;
mod editor;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use std::path::Path;

    use eframe::egui;

    let services = core_services::Services::native();
    let fixture = std::env::args().nth(1);

    eframe::run_native(
        "ascii-diagram-drawer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            // Deterministic rendering for golden-screenshot capture (gui-testing
            // skill): fixed dark theme. egui 0.37 has no animation toggle — the UI
            // is static (no animated widgets), so settled frames are stable.
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(app::create_app(
                services.clone(),
                fixture.as_deref().map(Path::new),
            )))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // No-op on wasm: the browser drives the app through the WebRunner glue added in
    // S-019. This entry exists so the binary target links; all app modules above are
    // compiled and type-checked for wasm (AC5).
}
