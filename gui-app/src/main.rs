//! ascii-diagram-drawer GUI entry point (ADR 008): eframe/egui on native desktop and
//! browser wasm. The app itself lives in [`app`]; this is bootstrap only.
//!
//! CLI arguments: an optional `.ad` file to load at startup (used by the visual test
//! tiers, gui-testing skill) and `--collections <dir>` to discover user shape
//! collections (`*.adc`) at startup (S-016 AC2). Invalid collection files are skipped
//! with a warning — startup always succeeds.
//!
//! Wasm note (S-012): AC5 only requires that the app code *compiles* for
//! `wasm32-unknown-unknown`; the real web entry point (`eframe::WebRunner` + a
//! `#[wasm_bindgen]` glue struct, per eframe's documented pattern) lands in S-019.

mod app;
mod canvas;
mod editor;
mod exportio;
mod fileio;
#[cfg(test)]
mod gui_tests;
#[cfg(test)]
mod screenshot;
mod sidebar;
mod template_editor;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use std::path::Path;

    use eframe::egui;

    let args = parse_startup_args(std::env::args().skip(1).collect());
    let (services, warnings) = core_services::Services::native_with_collections(
        args.collections_dir.as_deref().map(Path::new),
    );

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
                args.fixture.as_deref().map(Path::new),
                warnings.iter().map(|w| w.to_notice()).collect(),
            )))
        }),
    )
}

/// Startup arguments: an optional fixture `.ad` path + an optional collections dir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupArgs {
    /// A `.ad` file to load at startup (first positional argument).
    fixture: Option<String>,
    /// A directory of `*.adc` collection files (`--collections <dir>`).
    collections_dir: Option<String>,
}

/// Parses the CLI arguments; unknown flags and a missing `--collections` value are
/// ignored (startup always succeeds — S-016 AC2 philosophy).
#[must_use]
pub fn parse_startup_args(args: Vec<String>) -> StartupArgs {
    let mut out = StartupArgs::default();
    let mut pending_collections = false;
    for arg in args {
        if pending_collections {
            out.collections_dir = Some(arg);
            pending_collections = false;
        } else if arg == "--collections" {
            pending_collections = true;
        } else if out.fixture.is_none() && !arg.starts_with('-') {
            out.fixture = Some(arg);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_startup_args;

    /// No arguments: empty startup (built-ins only, no fixture).
    #[test]
    fn no_args_is_empty() {
        let args = parse_startup_args(Vec::new());
        assert_eq!(args.fixture, None);
        assert_eq!(args.collections_dir, None);
    }

    /// A positional argument is the startup fixture; `--collections` takes its value.
    #[test]
    fn fixture_and_collections_are_parsed() {
        let args = parse_startup_args(vec![
            "diagram.ad".into(),
            "--collections".into(),
            "/home/u/shapes".into(),
        ]);
        assert_eq!(args.fixture.as_deref(), Some("diagram.ad"));
        assert_eq!(args.collections_dir.as_deref(), Some("/home/u/shapes"));
    }

    /// Unknown flags and a dangling `--collections` are ignored, not fatal.
    #[test]
    fn unknown_flags_are_ignored() {
        let args = parse_startup_args(vec![
            "--wat".into(),
            "a.ad".into(),
            "b.ad".into(),
            "--collections".into(),
        ]);
        assert_eq!(args.fixture.as_deref(), Some("a.ad"));
        assert_eq!(args.collections_dir, None);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // No-op on wasm: the browser drives the app through the WebRunner glue added in
    // S-019. This entry exists so the binary target links; all app modules above are
    // compiled and type-checked for wasm (AC5).
}
