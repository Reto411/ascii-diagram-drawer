//! In-memory fakes for the ADR 009 seam — the T7 testing pattern ([§6.3](../../../doc/arc42/06-quality-model.md)).
//!
//! `gui-app` tests hold `Arc<dyn …>` services exactly like production, but backed by
//! these instead of disk/pipeline/rasterizer. Public (not `#[cfg(test)]`) on purpose:
//! the fakes live in `core-services` so GUI tests in another crate can use them
//! without pulling in any context crate's behavior.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

use diagram_model::Diagram;
use render_pipeline::{RenderError, ResolvedGrid};
use ui_api::{AnchorInfo, DiagramStore, Exporter, Renderer, ShapeCatalog, ShapeInfo, StoreError};

/// In-memory store: seed documents, load/save against the map. The same semantics as
/// the production `MemoryStore` (unknown path → IO error), but with a public seed API.
#[derive(Debug, Default)]
pub struct FakeStore {
    documents: Mutex<HashMap<PathBuf, Diagram>>,
}

impl FakeStore {
    /// Seeds `diagram` under `path`.
    #[must_use]
    pub fn with_document(self, path: impl Into<PathBuf>, diagram: Diagram) -> Self {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.into(), diagram);
        self
    }
}

impl DiagramStore for FakeStore {
    fn load(&self, path: &std::path::Path) -> Result<Diagram, StoreError> {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
            .ok_or_else(|| StoreError::Io {
                message: format!("no such document: {}", path.display()),
            })
    }

    fn save(&self, path: &std::path::Path, diagram: &Diagram) -> Result<(), StoreError> {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.to_path_buf(), diagram.clone());
        Ok(())
    }
}

/// Renderer that always returns one configured grid (or a typed error).
#[derive(Debug)]
pub struct FakeRenderer {
    outcome: Result<ResolvedGrid, RenderError>,
}

impl FakeRenderer {
    /// Always renders `grid`, whatever the input.
    #[must_use]
    pub fn with_grid(grid: ResolvedGrid) -> Self {
        Self { outcome: Ok(grid) }
    }

    /// Always fails with `UnknownShapeType` — for GUI error-path tests.
    #[must_use]
    pub fn failing(type_id: impl Into<String>) -> Self {
        Self {
            outcome: Err(RenderError::UnknownShapeType {
                type_id: type_id.into(),
            }),
        }
    }
}

impl Renderer for FakeRenderer {
    fn render(&self, _diagram: &Diagram) -> Result<ResolvedGrid, RenderError> {
        self.outcome.clone()
    }
}

/// Exporter that returns fixed media — deterministic bytes for GUI assertions.
#[derive(Debug)]
pub struct FakeExporter;

impl Exporter for FakeExporter {
    fn to_text(&self, _grid: &ResolvedGrid) -> String {
        "+---+\n|FAKE|\n+---+".into()
    }

    fn to_svg(&self, _grid: &ResolvedGrid) -> Result<String, ui_api::ExportError> {
        Ok("<svg xmlns=\"http://www.w3.org/2000/svg\">fake</svg>".into())
    }

    fn to_png(&self, _grid: &ResolvedGrid) -> Result<Vec<u8>, ui_api::ExportError> {
        Ok(b"\x89PNG\r\n\x1a\nfake".to_vec())
    }
}

/// Catalog seeded with explicit read models — no registry involved.
#[derive(Debug, Default)]
pub struct FakeCatalog {
    entries: BTreeMap<String, (ShapeInfo, Vec<AnchorInfo>)>,
}

impl FakeCatalog {
    /// Adds a shape with its anchors under `info.type_id`.
    #[must_use]
    pub fn add(mut self, info: ShapeInfo, anchors: Vec<AnchorInfo>) -> Self {
        self.entries.insert(info.type_id.clone(), (info, anchors));
        self
    }
}

impl ShapeCatalog for FakeCatalog {
    fn list(&self) -> Vec<ShapeInfo> {
        self.entries
            .values()
            .map(|(info, _)| info.clone())
            .collect()
    }

    fn get(&self, type_id: &str) -> Option<ShapeInfo> {
        self.entries.get(type_id).map(|(info, _)| info.clone())
    }

    fn anchors(&self, type_id: &str) -> Option<Vec<AnchorInfo>> {
        self.entries
            .get(type_id)
            .map(|(_, anchors)| anchors.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::geometry::{CellPos, CellSize};

    /// Every fake answers through its trait object — the full T7 surface a GUI test
    /// can rely on: store round-trip, fixed exports (all three media), catalog listing
    /// with anchors, and missing-key behavior.
    #[test]
    fn fakes_answer_through_their_traits() {
        // Store: seed + save both make documents loadable; unknown paths are IO errors.
        let diagram = Diagram::new();
        let store = FakeStore::default().with_document("seeded.ad", diagram.clone());
        store
            .save(std::path::Path::new("saved.ad"), &diagram)
            .expect("fake save");
        assert_eq!(
            store
                .load(std::path::Path::new("seeded.ad"))
                .expect("seeded"),
            diagram
        );
        assert_eq!(
            store.load(std::path::Path::new("saved.ad")).expect("saved"),
            diagram
        );
        assert!(matches!(
            store.load(std::path::Path::new("missing.ad")),
            Err(StoreError::Io { .. })
        ));

        // Renderer: both constructors, through `dyn Renderer`.
        let ok: Box<dyn Renderer> = Box::new(FakeRenderer::with_grid(
            ResolvedGrid::from_rows(&["|"]).expect("valid grid"),
        ));
        assert_eq!(ok.render(&diagram).expect("grid").to_text(), "|");
        let bad: Box<dyn Renderer> = Box::new(FakeRenderer::failing("nope"));
        assert!(matches!(
            bad.render(&diagram),
            Err(RenderError::UnknownShapeType { .. })
        ));

        // Exporter: all three media return their fixed payloads.
        let exporter = FakeExporter;
        let grid = ResolvedGrid::from_rows(&["|"]).expect("valid grid");
        assert_eq!(exporter.to_text(&grid), "+---+\n|FAKE|\n+---+");
        assert!(exporter.to_svg(&grid).expect("svg").contains("fake"));
        assert_eq!(&exporter.to_png(&grid).expect("png")[..4], b"\x89PNG");

        // Catalog: list/get/anchors, including the empty-anchors and missing cases.
        let info = ShapeInfo {
            type_id: "box".into(),
            name: "Box".into(),
            size: CellSize::new(9, 5).expect("valid size"),
        };
        let anchor = AnchorInfo {
            name: "n".into(),
            position: CellPos::new(4, 0),
        };
        let catalog = FakeCatalog::default()
            .add(info.clone(), vec![anchor.clone()])
            .add(
                ShapeInfo {
                    type_id: "ellipse".into(),
                    name: "Ellipse".into(),
                    size: CellSize::new(7, 4).expect("valid size"),
                },
                Vec::new(),
            );
        assert_eq!(catalog.list().len(), 2);
        assert_eq!(catalog.get("box"), Some(info));
        assert_eq!(catalog.anchors("box"), Some(vec![anchor]));
        assert_eq!(catalog.anchors("ellipse"), Some(Vec::new()));
        assert_eq!(catalog.get("nope"), None);
        assert_eq!(catalog.anchors("nope"), None);
    }
}
