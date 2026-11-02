//! ascii-diagram-drawer GUI (ADR 008, S-019): the eframe/egui presentation layer,
//! shared by both entry points — the native desktop binary (`run_native`) and the
//! browser wasm glue ([`web::WebHandle`]). The app itself lives in [`app`].
//!
//! Startup arguments (native CLI): an optional `.ad` file to load at startup (used
//! by the visual test tiers, gui-testing skill) and `--collections <dir>` to discover
//! user shape collections (`*.adc`) at startup (S-016 AC2). Invalid collection files
//! are skipped with a warning — startup always succeeds.

pub mod app;
pub mod appbar;
pub mod canvas;
pub mod config;
pub mod dialogs;
pub mod editor;
pub mod exportio;
pub mod fileio;
#[cfg(test)]
mod gui_tests;
#[cfg(test)]
mod screenshot;
pub mod sidebar;
pub mod template_editor;
pub mod theme;
#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(target_arch = "wasm32")]
pub mod webio;

/// Startup arguments: an optional fixture `.ad` path + an optional collections dir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupArgs {
    /// A `.ad` file to load at startup (first positional argument).
    pub(crate) fixture: Option<String>,
    /// A directory of `*.adc` collection files (`--collections <dir>`).
    pub(crate) collections_dir: Option<String>,
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

/// Native entry point (called from the binary): bootstraps the services and runs
/// eframe with the undecorated Material window (FR14 / ADR 017).
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() -> eframe::Result<()> {
    use std::path::Path;

    use eframe::egui;

    let args = parse_startup_args(std::env::args().skip(1).collect());
    let (services, warnings) = core_services::Services::native_with_collections(
        args.collections_dir.as_deref().map(Path::new),
    );

    eframe::run_native(
        "ascii-diagram-drawer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1024.0, 768.0])
                // FR14 (CR-006 / ADR 017): the window runs undecorated — the custom
                // Material app bar is the window border itself (drag region + window
                // controls). Wasm keeps the browser's own chrome (NFR1); this builder
                // only exists on native.
                .with_decorations(false),
            ..Default::default()
        },
        Box::new(|cc| {
            // FR13 (CR-005) / FR14 (CR-006, ADR 017): the theme comes from the
            // persisted app config (default native — follows the OS/browser
            // appearance, dark where unknown). Install the Material palettes on the
            // eframe context BEFORE the first UI pass: egui snapshots the active
            // style into each Ui at creation, so a mid-pass install would render
            // frame 1 with default spacing and settle over two frames (a visible
            // jump in the chrome height at startup).
            let config =
                crate::config::AppConfig::load(crate::config::default_config_path().as_deref());
            crate::theme::install(&cc.egui_ctx, config.theme);
            Ok(Box::new(app::create_app(
                services.clone(),
                config,
                args.fixture.as_deref().map(Path::new),
                warnings.iter().map(|w| w.to_notice()).collect(),
            )))
        }),
    )
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
