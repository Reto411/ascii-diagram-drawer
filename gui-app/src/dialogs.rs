//! System file dialogs (S-039 / CR-001) — the seam between the GUI and the OS.
//!
//! The File-menu actions ask a [`FileDialogBackend`] for a path; on native hosts
//! the default backend is the XDG document portal via `rfd` (ADR 018 — no GTK
//! dependency). Where no dialog transport exists (wasm target, or a native host
//! without a D-Bus session) the backend reports [`DialogResult::Unavailable`] and
//! the app falls back to its inline path prompt / in-app format choice (NFR1:
//! the browser keeps today's behavior).

use std::path::{Path, PathBuf};

/// The kind of dialog requested — one per File-menu action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    /// Open an existing `.ad` document.
    Open,
    /// Save the diagram document (`.ad`).
    Save,
    /// Save as: the user selects what to save — `.ad` document or `.txt` text output.
    SaveAs,
    /// Export: the user selects the format — SVG or PNG.
    Export,
}

/// The outcome of a dialog round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    /// The user picked a path. Which kind to write is derived from its extension
    /// (see [`save_as_kind_for`] / [`export_format_for`]).
    Chosen(PathBuf),
    /// The user cancelled — the caller does nothing.
    Cancelled,
    /// No usable dialog transport (wasm target, no D-Bus session). The caller
    /// falls back to the inline prompt flow.
    Unavailable(String),
}

/// Seam: shows a system file dialog and reports the outcome.
///
/// Native default: [`RfdBackend`]. GUI tests inject fakes; wasm has no backend
/// at all (the app treats a missing backend as `Unavailable`).
pub trait FileDialogBackend: Send + Sync {
    fn show(&self, kind: DialogKind, default_file: Option<&Path>) -> DialogResult;
}

// ── Pure path semantics — unit-testable without any dialog ───────────────────

/// What a Save As selection means (AC3): `.txt` writes the rendered text output,
/// anything else is the diagram document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAsKind {
    /// The `.ad` diagram document (the canonical save target).
    Document,
    /// The exact canvas bytes as a `.txt` file (an export of the document).
    Text,
}

/// The export format a path asks for (AC4): `.png` is PNG, anything else SVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Svg,
    Png,
}

fn extension_is(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Ensure `path` carries one of the dialog's extensions; when it has none (or an
/// unrecognized one) the preferred default is appended, so a bare "diagram" typed
/// into the Save As dialog becomes `diagram.ad`.
fn with_default_extension(path: &Path, preferred: &str, recognized: &[&str]) -> PathBuf {
    if recognized.iter().any(|ext| extension_is(path, ext)) {
        path.to_path_buf()
    } else {
        let mut p = path.to_path_buf();
        p.set_extension(preferred);
        p
    }
}

/// Interprets a Save As selection: the normalized sink path plus what it means.
#[must_use]
pub fn save_as_kind_for(path: &Path) -> (SaveAsKind, PathBuf) {
    if extension_is(path, "txt") {
        (SaveAsKind::Text, path.to_path_buf())
    } else {
        (
            SaveAsKind::Document,
            with_default_extension(path, "ad", &["ad", "txt"]),
        )
    }
}

/// Interprets an Export selection: the normalized sink path plus the format.
#[must_use]
pub fn export_format_for(path: &Path) -> (ExportFormat, PathBuf) {
    if extension_is(path, "png") {
        (ExportFormat::Png, path.to_path_buf())
    } else {
        (
            ExportFormat::Svg,
            with_default_extension(path, "svg", &["svg", "png"]),
        )
    }
}

// ── The native default backend (rfd on the XDG document portal) ─────────────

/// The native default: `rfd` talking to the XDG document portal over D-Bus —
/// no GTK dependency, so it builds in the isolated devcontainer (ADR 018).
#[cfg(not(target_arch = "wasm32"))]
pub struct RfdBackend;

/// Whether this host has a transport the portal backend can use. The xdg-portal
/// backend needs a D-Bus *session* (with the portal service); `DISPLAY` alone is
/// not enough — without a session rfd would fall back to spawning `zenity`,
/// which we do not rely on. Checked before any dialog call so headless hosts
/// fail fast into the inline-prompt fallback instead of attempting D-Bus.
#[cfg(not(target_arch = "wasm32"))]
fn host_has_dialog_transport() -> bool {
    std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
}

#[cfg(not(target_arch = "wasm32"))]
impl FileDialogBackend for RfdBackend {
    fn show(&self, kind: DialogKind, default_file: Option<&Path>) -> DialogResult {
        if !host_has_dialog_transport() {
            return DialogResult::Unavailable(
                "no D-Bus session — system dialogs unavailable".to_owned(),
            );
        }
        // rfd's builder methods consume and return the dialog, so each arm is one
        // chain ending in the blocking call.
        let mut d = rfd::FileDialog::new();
        let picked = match kind {
            DialogKind::Open => d
                .set_title("Open diagram")
                .add_filter("Diagram document", &["ad"])
                .pick_file(),
            // Save is prefilled with the current file (name + directory) so the
            // dialog opens on it; the user may still pick a different path.
            DialogKind::Save => {
                if let Some(current) = default_file {
                    if let Some(parent) = current.parent().filter(|p| !p.as_os_str().is_empty()) {
                        d = d.set_directory(parent);
                    }
                    if let Some(name) = current.file_name().and_then(|n| n.to_str()) {
                        d = d.set_file_name(name);
                    }
                }
                d.set_title("Save diagram")
                    .add_filter("Diagram document", &["ad"])
                    .save_file()
            }
            // Save As / Export: one dialog, the choice (what to save / which
            // format) is the file-type filter; the picked path's extension
            // decides what is written.
            DialogKind::SaveAs => d
                .set_title("Save as")
                .add_filter("Diagram document", &["ad"])
                .add_filter("Text output", &["txt"])
                .save_file(),
            DialogKind::Export => d
                .set_title("Export diagram")
                .add_filter("SVG image", &["svg"])
                .add_filter("PNG image", &["png"])
                .save_file(),
        };
        picked.map_or(DialogResult::Cancelled, DialogResult::Chosen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_as_txt_path_is_text_output() {
        assert_eq!(
            save_as_kind_for(Path::new("/tmp/out.txt")),
            (SaveAsKind::Text, PathBuf::from("/tmp/out.txt"))
        );
        // Case-insensitive extension.
        assert_eq!(
            save_as_kind_for(Path::new("/tmp/OUT.TXT")).0,
            SaveAsKind::Text
        );
    }

    #[test]
    fn save_as_other_or_missing_extension_is_the_document() {
        assert_eq!(
            save_as_kind_for(Path::new("/tmp/diagram.ad")),
            (SaveAsKind::Document, PathBuf::from("/tmp/diagram.ad"))
        );
        // No extension → the preferred default is appended.
        assert_eq!(
            save_as_kind_for(Path::new("/tmp/diagram")),
            (SaveAsKind::Document, PathBuf::from("/tmp/diagram.ad"))
        );
        // Unrecognized extension → replaced by the document default.
        assert_eq!(
            save_as_kind_for(Path::new("/tmp/diagram.json")),
            (SaveAsKind::Document, PathBuf::from("/tmp/diagram.ad"))
        );
    }

    #[test]
    fn export_png_path_is_png() {
        assert_eq!(
            export_format_for(Path::new("/tmp/pic.png")),
            (ExportFormat::Png, PathBuf::from("/tmp/pic.png"))
        );
        assert_eq!(
            export_format_for(Path::new("/tmp/PI.PNG")).0,
            ExportFormat::Png
        );
    }

    #[test]
    fn export_other_or_missing_extension_is_svg() {
        assert_eq!(
            export_format_for(Path::new("/tmp/pic.svg")),
            (ExportFormat::Svg, PathBuf::from("/tmp/pic.svg"))
        );
        assert_eq!(
            export_format_for(Path::new("/tmp/pic")),
            (ExportFormat::Svg, PathBuf::from("/tmp/pic.svg"))
        );
        assert_eq!(
            export_format_for(Path::new("/tmp/pic.pdf")),
            (ExportFormat::Svg, PathBuf::from("/tmp/pic.svg"))
        );
    }

    // Native-only: `RfdBackend` does not exist on wasm (no system dialogs —
    // S-019 / ADR 018).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rfd_backend_is_unavailable_without_a_dbus_session() {
        // Never touch the environment (tests run in parallel and share it). On a
        // desktop with a live session bus a real dialog would open, so skip there;
        // headless hosts (the devcontainer) always take this branch.
        if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
            eprintln!("skipping: a D-Bus session is present (a real dialog would open)");
            return;
        }
        let res = RfdBackend.show(DialogKind::Open, None);
        assert!(
            matches!(res, DialogResult::Unavailable(_)),
            "headless host must report Unavailable: {res:?}"
        );
    }
}
