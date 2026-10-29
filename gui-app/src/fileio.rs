//! File open/save + text-output flows (S-014) — pure logic over the ADR 009 seam,
//! unit-testable headlessly against fakes (T7). The egui glue in [`crate::app`] only
//! calls these and displays their results; no file access happens in views.
//!
//! Rules recorded here (story Implementation notes):
//! - **Open** replaces the model only on success; parse/IO errors keep the previous
//!   model and surface a notice with line/column (RS2). A lossy load (newer minor
//!   version) is reported, never silent.
//! - **Save** proceeds immediately unless the current document came from a file with
//!   skipped unknown statements — then the GUI must confirm first (AC4: no silent
//!   data loss; the seam cannot preserve-on-resave, ADR 009 v4).
//! - **Text output** is exactly `Exporter::to_text(Renderer::render(model))` — the
//!   same bytes the canvas draws (WYSIWYG by construction, AC3).

use std::path::{Path, PathBuf};

use core_services::Services;
use diagram_model::Diagram;
use ui_api::{DiagramStore, LoadedDocument, StoreError};

/// What an open attempt did (AC1) — drives the status bar and the tests. The model is
/// replaced by the caller only on [`OpenOutcome::Loaded`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    /// The document loaded; `skipped_unknowns > 0` means newer-version statements were
    /// dropped at load and will be lost on save (RS2 notice + AC4 warning armed).
    Loaded {
        path: PathBuf,
        diagram: Diagram,
        skipped_unknowns: usize,
    },
    /// The file's content is not a valid `.ad` document — previous model kept.
    ParseError {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    /// Platform IO failed (missing file, unreadable) — previous model kept.
    IoError { path: PathBuf, message: String },
}

/// Runs the open flow against the store seam (AC1): parse errors and IO errors are
/// mapped to notices; nothing here mutates any model.
#[must_use]
pub fn open(store: &dyn DiagramStore, path: &Path) -> OpenOutcome {
    match store.load(path) {
        Ok(LoadedDocument {
            diagram,
            skipped_unknowns,
        }) => OpenOutcome::Loaded {
            path: path.to_path_buf(),
            diagram,
            skipped_unknowns,
        },
        Err(StoreError::Parse {
            line,
            column,
            message,
        }) => OpenOutcome::ParseError {
            path: path.to_path_buf(),
            line,
            column,
            message,
        },
        Err(StoreError::Io { message }) => OpenOutcome::IoError {
            path: path.to_path_buf(),
            message,
        },
    }
}

/// One-line status-bar text for an open outcome.
#[must_use]
pub fn open_message(outcome: &OpenOutcome) -> String {
    match outcome {
        OpenOutcome::Loaded {
            path,
            skipped_unknowns,
            ..
        } if *skipped_unknowns > 0 => format!(
            "Opened {} — WARNING: {skipped_unknowns} newer-version statement(s) were skipped and will be lost on save",
            path.display()
        ),
        OpenOutcome::Loaded { path, .. } => format!("Opened {}", path.display()),
        OpenOutcome::ParseError {
            path,
            line,
            column,
            message,
        } => format!(
            "Open failed — {} (line {line}, column {column}): {message} — previous diagram kept",
            path.display()
        ),
        OpenOutcome::IoError { path, message } => format!(
            "Open failed — {}: {message} — previous diagram kept",
            path.display()
        ),
    }
}

/// Whether a save may proceed immediately or needs user confirmation first (AC4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveDecision {
    /// Nothing at risk — write.
    Proceed,
    /// The current document came from a file with `count` skipped unknown statements;
    /// saving drops them — the GUI must confirm before overwriting.
    ConfirmDrop { count: usize },
}

/// The AC4 rule: warn exactly when the loaded document carries data the save cannot
/// keep (ADR 009 v4 — the seam cannot preserve-on-resave).
#[must_use]
pub fn decide_save(skipped_unknowns: usize) -> SaveDecision {
    if skipped_unknowns == 0 {
        SaveDecision::Proceed
    } else {
        SaveDecision::ConfirmDrop {
            count: skipped_unknowns,
        }
    }
}

/// What a save attempt did (AC2) — drives the status bar and the tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// Written to `path`.
    Saved { path: PathBuf },
    /// The AC4 warning fired; nothing was written until confirmed.
    ConfirmationNeeded { count: usize },
    /// Platform IO failed (RS5 — surfaced, never silent).
    IoError { path: PathBuf, message: String },
}

/// Runs the save flow against the store seam: decides (AC4), then writes on
/// [`SaveDecision::Proceed`] or when `confirmed` is set.
#[must_use]
pub fn save(
    store: &dyn DiagramStore,
    path: &Path,
    diagram: &Diagram,
    skipped_unknowns: usize,
    confirmed: bool,
) -> SaveOutcome {
    match decide_save(skipped_unknowns) {
        SaveDecision::ConfirmDrop { count } if !confirmed => {
            return SaveOutcome::ConfirmationNeeded { count };
        }
        _ => {}
    }
    match store.save(path, diagram) {
        Ok(()) => SaveOutcome::Saved {
            path: path.to_path_buf(),
        },
        Err(StoreError::Io { message }) => SaveOutcome::IoError {
            path: path.to_path_buf(),
            message,
        },
        // Saving our own canonical model cannot fail to parse — the seam reports it
        // here for forward compatibility; surface it like any other failure (RS5).
        Err(StoreError::Parse {
            line,
            column,
            message,
        }) => SaveOutcome::IoError {
            path: path.to_path_buf(),
            message: format!("line {line}, column {column}: {message}"),
        },
    }
}

/// One-line status-bar text for a save outcome.
#[must_use]
pub fn save_message(outcome: &SaveOutcome) -> String {
    match outcome {
        SaveOutcome::Saved { path } => format!("Saved {}", path.display()),
        SaveOutcome::ConfirmationNeeded { count } => format!(
            "WARNING: saving drops {count} newer-version statement(s) — confirm to overwrite"
        ),
        SaveOutcome::IoError { path, message } => {
            format!("Save failed — {}: {message}", path.display())
        }
    }
}

/// The exact text output for the model (AC3): `to_text(render(model))` — by
/// construction the same bytes the canvas draws (WYSIWYG).
pub fn text_output(services: &Services, diagram: &Diagram) -> Result<String, String> {
    let grid = services
        .renderer
        .render(diagram)
        .map_err(|e| e.to_string())?;
    Ok(services.exporter.to_text(&grid))
}

/// The `.txt` sink path for a document file (AC3): same stem, `.txt` extension.
#[must_use]
pub fn text_output_path(document: &Path) -> PathBuf {
    document.with_extension("txt")
}

/// Writes an auxiliary text output to `path` (the AC3 file sink). This is the single
/// sanctioned direct-file-access helper in gui-app — `.ad` documents still go through
/// the store seam exclusively (ADR 009 v4 notes). Native only: on web targets the
/// browser download replaces this call (S-019).
#[cfg(not(target_arch = "wasm32"))]
pub fn write_text_file(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|err| format!("{}: {err}", path.display()))
}

/// Web target: no filesystem — the browser download lands in S-019.
#[cfg(target_arch = "wasm32")]
pub fn write_text_file(_path: &Path, _text: &str) -> Result<(), String> {
    Err("not available on web — the text download lands with the wasm milestone (S-019)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::DrawingApp;
    use core_services::fakes::FakeStore;
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::ShapeInstance;
    use std::sync::Arc;

    /// A one-box diagram (the same shape the built-in catalog serves).
    fn one_box() -> Diagram {
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_shape(
                ShapeInstance::new(
                    id,
                    "box",
                    "",
                    CellPos::origin(),
                    CellSize::new(9, 5).expect("valid size"),
                )
                .expect("valid instance"),
            )
            .expect("placement");
        diagram
    }

    /// A store whose `load` always fails with a fixed parse error (AC1 mapping test).
    struct ParseFailingStore {
        line: usize,
        column: usize,
        message: String,
    }

    impl DiagramStore for ParseFailingStore {
        fn load(&self, _path: &Path) -> Result<LoadedDocument, StoreError> {
            Err(StoreError::Parse {
                line: self.line,
                column: self.column,
                message: self.message.clone(),
            })
        }
        fn save(&self, _path: &Path, _diagram: &Diagram) -> Result<(), StoreError> {
            Ok(())
        }
    }

    /// A store whose `save` always fails with a fixed IO error (AC2/RS5 test).
    struct SaveFailingStore {
        message: String,
    }

    impl DiagramStore for SaveFailingStore {
        fn load(&self, _path: &Path) -> Result<LoadedDocument, StoreError> {
            Ok(LoadedDocument::lossless(Diagram::new()))
        }
        fn save(&self, _path: &Path, _diagram: &Diagram) -> Result<(), StoreError> {
            Err(StoreError::Io {
                message: self.message.clone(),
            })
        }
    }

    /// A store whose `save` fails with a parse error (the seam's forward-compat arm —
    /// cannot happen for our canonical model, but must be surfaced, not panicked on).
    struct SaveParseFailingStore {
        line: usize,
        column: usize,
        message: String,
    }

    impl DiagramStore for SaveParseFailingStore {
        fn load(&self, _path: &Path) -> Result<LoadedDocument, StoreError> {
            Ok(LoadedDocument::lossless(Diagram::new()))
        }
        fn save(&self, _path: &Path, _diagram: &Diagram) -> Result<(), StoreError> {
            Err(StoreError::Parse {
                line: self.line,
                column: self.column,
                message: self.message.clone(),
            })
        }
    }

    /// App wired to a fake store holding `doc` at `path` (plus optional skipped count).
    fn app_with(path: &str, doc: Diagram, skipped: usize) -> DrawingApp {
        let mut store = FakeStore::default().with_document(path, doc);
        if skipped > 0 {
            store = store.with_skipped_unknowns(path, skipped);
        }
        DrawingApp::new(Services {
            store: Arc::new(store),
            ..Services::native()
        })
    }

    /// AC1 — open success replaces the model and records the file.
    #[test]
    fn open_success_replaces_the_model() {
        let mut app = app_with("a.ad", one_box(), 0);
        // Pre-existing content that must be replaced.
        app.diagram = Diagram::new();

        let outcome = app.open(Path::new("a.ad"));
        match &outcome {
            OpenOutcome::Loaded {
                diagram,
                skipped_unknowns: 0,
                ..
            } => {
                assert_eq!(*diagram, one_box());
            }
            other => panic!("expected lossless load, got {other:?}"),
        }
        assert_eq!(app.diagram.len(), 1);
        assert_eq!(
            app.current_file().map(|p| p.as_path()),
            Some(Path::new("a.ad"))
        );
    }

    /// AC1 — parse error: notice carries line/column, previous model kept (RS2).
    #[test]
    fn open_parse_error_keeps_the_previous_model() {
        let services = Services::native();
        let mut app = DrawingApp::new(Services {
            store: Arc::new(ParseFailingStore {
                line: 3,
                column: 7,
                message: "expected keyword".to_owned(),
            }),
            ..services
        });
        app.diagram = one_box();

        let outcome = app.open(Path::new("bad.ad"));
        assert_eq!(
            outcome,
            OpenOutcome::ParseError {
                path: PathBuf::from("bad.ad"),
                line: 3,
                column: 7,
                message: "expected keyword".to_owned(),
            }
        );
        assert_eq!(app.diagram.len(), 1, "previous model kept");
        let message = app.status.clone();
        assert!(message.contains("line 3, column 7"), "{message}");
        assert!(message.contains("previous diagram kept"), "{message}");
    }

    /// AC1 — IO error: previous model kept, failure surfaced (RS5).
    #[test]
    fn open_io_error_keeps_the_previous_model() {
        let mut app = app_with("a.ad", one_box(), 0);
        let _ = app.open(Path::new("a.ad"));

        let outcome = app.open(Path::new("missing.ad"));
        assert!(matches!(outcome, OpenOutcome::IoError { .. }));
        assert_eq!(app.diagram.len(), 1, "previous model kept");
        assert!(
            app.status.contains("previous diagram kept"),
            "{:?}",
            app.status
        );
    }

    /// AC4 — lossy load arms the save warning; the first Save must not write.
    #[test]
    fn lossy_load_arms_the_save_confirmation() {
        let mut app = app_with("newer.ad", one_box(), 2);
        assert!(matches!(
            app.open(Path::new("newer.ad")),
            OpenOutcome::Loaded {
                skipped_unknowns: 2,
                ..
            }
        ));

        // First save attempt: warning only — nothing written yet.
        let outcome = app.save_requested();
        assert_eq!(outcome, SaveOutcome::ConfirmationNeeded { count: 2 });
        assert!(app.confirming_save());
        assert!(app.status.contains("confirm"), "{:?}", app.status);

        // Cancel: the warning is dismissed, still nothing written.
        app.cancel_confirmation();
        assert!(!app.confirming_save());

        // New attempt + confirm: now it writes.
        let _ = app.save_requested();
        let outcome = app.confirm_save();
        assert!(matches!(outcome, SaveOutcome::Saved { .. }));
        assert!(!app.confirming_save());
    }

    /// AC4 — clean load saves immediately, no confirmation round trip.
    #[test]
    fn clean_load_saves_without_confirmation() {
        let mut app = app_with("a.ad", one_box(), 0);
        let _ = app.open(Path::new("a.ad"));

        let outcome = app.save_requested();
        assert!(matches!(outcome, SaveOutcome::Saved { .. }));
        assert!(!app.confirming_save());
    }

    /// The seam's parse-error-on-save arm is surfaced like any other failure (RS5).
    #[test]
    fn save_parse_error_is_surfaced_with_location() {
        let outcome = save(
            &SaveParseFailingStore {
                line: 2,
                column: 3,
                message: "cannot happen".to_owned(),
            },
            Path::new("x.ad"),
            &one_box(),
            0,
            true,
        );
        assert!(matches!(outcome, SaveOutcome::IoError { .. }));
        let message = save_message(&outcome);
        assert!(message.contains("line 2, column 3"), "{message}");
    }

    /// AC2 — save failure surfaces a clear notice (RS5); the model is untouched.
    #[test]
    fn save_io_error_is_surfaced() {
        let outcome = save(
            &SaveFailingStore {
                message: "disk full".to_owned(),
            },
            Path::new("x.ad"),
            &one_box(),
            0,
            true,
        );
        assert_eq!(
            outcome,
            SaveOutcome::IoError {
                path: PathBuf::from("x.ad"),
                message: "disk full".to_owned(),
            }
        );
        assert!(save_message(&outcome).starts_with("Save failed"));
    }

    /// AC3 — the text output equals exactly what the canvas draws (WYSIWYG pin).
    #[test]
    fn text_output_is_the_exact_canvas_bytes() {
        let mut app = app_with("a.ad", one_box(), 0);
        let _ = app.open(Path::new("a.ad"));

        let canvas = app.canvas_frame().expect("renders");
        assert_eq!(app.text_output().expect("text"), canvas.0);
        // A lone 9×5 box renders as its border — the exact expected bytes.
        let lines: Vec<&str> = canvas.0.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(lines[0].contains("+-------+"));
    }

    /// AC3 — saving `.txt` writes exactly the output bytes next to the document.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn save_text_writes_the_exact_bytes_to_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let document = dir.path().join("diagram.ad");
        std::fs::write(&document, "ad 1.0\nscale 1\n").expect("seed");

        let mut app = app_with(document.to_str().expect("utf8"), one_box(), 0);
        let _ = app.open(&document);

        let outcome = app.save_text();
        assert!(outcome.is_ok(), "{outcome:?}");
        let txt_path = dir.path().join("diagram.txt");
        assert_eq!(
            std::fs::read_to_string(&txt_path).expect("txt written"),
            app.text_output().expect("text")
        );
    }

    /// AC3 — the `.txt` sink path derives from the document path.
    #[test]
    fn text_output_path_derives_the_txt_sibling() {
        assert_eq!(
            text_output_path(Path::new("dir/diagram.ad")),
            PathBuf::from("dir/diagram.txt")
        );
        assert_eq!(
            text_output_path(Path::new("diagram")),
            PathBuf::from("diagram.txt")
        );
    }

    /// The AC4 decision table.
    #[test]
    fn decide_save_warns_exactly_when_data_would_be_dropped() {
        assert_eq!(decide_save(0), SaveDecision::Proceed);
        assert_eq!(decide_save(1), SaveDecision::ConfirmDrop { count: 1 });
        assert_eq!(decide_save(9), SaveDecision::ConfirmDrop { count: 9 });
    }

    /// The lossless-load helper used by in-memory stores.
    #[test]
    fn lossless_helper_reports_zero_skipped() {
        let doc = LoadedDocument::lossless(one_box());
        assert_eq!(doc.skipped_unknowns, 0);
        assert_eq!(doc.diagram.len(), 1);
    }

    /// `write_text_file` reports failures with the path (RS5 at the sink).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn write_text_file_reports_failures_with_the_path() {
        let err = write_text_file(Path::new("/nonexistent-dir/x.txt"), "t").expect_err("must fail");
        assert!(err.contains("/nonexistent-dir/x.txt"), "{err}");
    }
}
