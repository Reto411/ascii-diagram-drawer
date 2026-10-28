//! Diagram store backends (ADR 009): filesystem for native targets, in-memory for
//! targets without one. Platform IO is confined to this module — the C3 rule.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use diagram_model::Diagram;
use ui_api::{DiagramStore, StoreError};

/// Maps an `ad-format` parse failure into the seam's vocabulary (line/column kept —
/// the GUI's error dialog jumps there).
fn map_parse(err: ad_format::ParseError) -> StoreError {
    StoreError::Parse {
        line: err.line,
        column: err.column,
        message: err.message,
    }
}

/// Filesystem-backed store (native targets): reads/writes `.ad` files. Stateless —
/// every call touches the disk at `path`.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsStore;

impl DiagramStore for FsStore {
    fn load(&self, path: &Path) -> Result<Diagram, StoreError> {
        let text = std::fs::read_to_string(path).map_err(|err| StoreError::Io {
            message: format!("{}: {err}", path.display()),
        })?;
        ad_format::parse(&text).map_err(map_parse)
    }

    fn save(&self, path: &Path, diagram: &Diagram) -> Result<(), StoreError> {
        std::fs::write(path, ad_format::serialize(diagram)).map_err(|err| StoreError::Io {
            message: format!("{}: {err}", path.display()),
        })
    }
}

/// In-memory store for targets without a filesystem (wasm): documents live in a map
/// keyed by their (virtual) path. `load` of an unknown key is reported as an IO error
/// — there is no filesystem to miss on, and the GUI shows the same message either way.
#[derive(Debug, Default)]
pub struct MemoryStore {
    documents: Mutex<HashMap<PathBuf, Diagram>>,
}

impl MemoryStore {
    /// Seeds a document under `path` (test helper; equivalent to a prior save).
    #[must_use]
    pub fn with_document(self, path: impl Into<PathBuf>, diagram: Diagram) -> Self {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.into(), diagram);
        self
    }
}

impl DiagramStore for MemoryStore {
    fn load(&self, path: &Path) -> Result<Diagram, StoreError> {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
            .ok_or_else(|| StoreError::Io {
                message: format!("no such document: {}", path.display()),
            })
    }

    fn save(&self, path: &Path, diagram: &Diagram) -> Result<(), StoreError> {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.to_path_buf(), diagram.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::{geometry::CellPos, objects::ShapeInstance};

    fn small_diagram() -> Diagram {
        let mut d = Diagram::new();
        // A single built-in box at the origin — valid through every invariant.
        let id = d.new_object_id();
        let instance = ShapeInstance::new(
            id,
            "box",
            "",
            CellPos::origin(),
            diagram_model::geometry::CellSize::new(9, 5).expect("valid size"),
        )
        .expect("valid instance");
        d.place_shape(instance).expect("valid placement");
        d
    }

    /// AC2 — round-trip through a real tempdir: save writes parseable `.ad`, load
    /// gives back an equal diagram; the QS7 lossless property holds at the seam.
    #[test]
    fn fs_store_round_trips_a_real_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sample.ad");
        let store = FsStore;

        let diagram = small_diagram();
        store.save(&path, &diagram).expect("save");
        let text = std::fs::read_to_string(&path).expect("file written");
        assert!(text.starts_with("ad 1.0"), "canonical header: {text:?}");

        let loaded = store.load(&path).expect("load");
        assert_eq!(loaded, diagram);
    }

    #[test]
    fn fs_store_load_reports_missing_files_as_io_errors() {
        let err = FsStore
            .load(Path::new("/nonexistent/x.ad"))
            .expect_err("must fail");
        assert!(matches!(err, StoreError::Io { .. }), "got {err:?}");
    }

    #[test]
    fn fs_store_load_reports_parse_errors_with_location() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bad.ad");
        // Malformed on purpose: a shape statement without its quoted label — the
        // parser must report exactly where it diverged (line 2, the `at` token at
        // column 11, where the quoted label was expected).
        std::fs::write(&path, "ad 1.0\nshape box at 0 0\n").expect("write");

        let err = FsStore.load(&path).expect_err("must fail");
        match err {
            StoreError::Parse { line, column, .. } => {
                assert_eq!((line, column), (2, 11));
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn memory_store_holds_documents_by_path() {
        let store = MemoryStore::default();
        let path = Path::new("docs/sample.ad");

        assert!(matches!(
            store.load(path).expect_err("empty store"),
            StoreError::Io { .. }
        ));

        let diagram = small_diagram();
        store.save(path, &diagram).expect("save");
        assert_eq!(store.load(path).expect("load"), diagram);
    }

    #[test]
    fn memory_store_seeding_matches_saving() {
        let diagram = small_diagram();
        let via_seed = MemoryStore::default().with_document("a.ad", diagram.clone());
        let via_save = MemoryStore::default();
        via_save.save(Path::new("a.ad"), &diagram).expect("save");

        assert_eq!(
            via_seed.load(Path::new("a.ad")).expect("seeded"),
            via_save.load(Path::new("a.ad")).expect("saved")
        );
    }
}
