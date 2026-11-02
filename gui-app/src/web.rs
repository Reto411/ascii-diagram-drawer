//! Browser entry glue (S-019): the `#[wasm_bindgen]` handle JavaScript calls to start
//! the app on a canvas — eframe's documented web pattern. The native binary is a
//! no-op on wasm; Trunk builds this library target into the web bundle.

use wasm_bindgen::prelude::*;

/// Handle to the web app from JavaScript (cloned cheaply, per eframe docs).
#[derive(Clone)]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[wasm_bindgen]
impl WebHandle {
    /// Installs the panic hook and web logger, then returns.
    #[expect(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        eframe::WebLogger::init(log::LevelFilter::Info).ok();
        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    /// Starts the app on `canvas` (call once from JavaScript). The web composition
    /// (`Services::web`) uses the session-memory store with download-on-save; options
    /// come from `localStorage`, and there is no startup fixture or collections dir.
    #[wasm_bindgen]
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    // Mirror the native startup (run_native): install the persisted theme
                    // before the first UI pass so frame 1 has the final spacing.
                    let config = crate::config::AppConfig::load_web();
                    crate::theme::install(&cc.egui_ctx, config.theme);
                    Ok(Box::new(crate::app::create_app(
                        core_services::Services::web(),
                        config,
                        None,
                        Vec::new(),
                    )))
                }),
            )
            .await
    }

    /// Shuts down eframe and cleans up resources.
    #[wasm_bindgen]
    pub fn destroy(&self) {
        self.runner.destroy();
    }
}
