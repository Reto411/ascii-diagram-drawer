//! In-memory fakes for the ADR 009 seam — the T7 testing pattern ([§6.3](../../../doc/arc42/06-quality-model.md)).
//!
//! `gui-app` tests hold `Arc<dyn …>` services exactly like production, but backed by
//! these instead of disk/pipeline/rasterizer. Public (not `#[cfg(test)]`) on purpose:
//! the fakes live in `core-services` so GUI tests in another crate can use them
//! without pulling in any context crate's behavior.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Mutex;

use diagram_model::{Diagram, geometry::CellPos, objects::ObjectId};
use render_pipeline::{RenderError, ResolvedGrid};
use ui_api::{
    AnchorInfo, CatalogError, DiagramStore, Exporter, LoadedDocument, NewTemplate, Renderer,
    ResizeRuleSpec, ShapeCatalog, ShapeInfo, StoreError,
};

use crate::registry_state::pad_rows;

/// In-memory store: seed documents, load/save against the map. The same semantics as
/// the production `MemoryStore` (unknown path → IO error), but with a public seed API.
#[derive(Debug, Default)]
pub struct FakeStore {
    documents: Mutex<HashMap<PathBuf, Diagram>>,
    skipped_unknowns: Mutex<HashMap<PathBuf, usize>>,
}

impl FakeStore {
    /// Seeds `diagram` under `path` (lossless load).
    #[must_use]
    pub fn with_document(self, path: impl Into<PathBuf>, diagram: Diagram) -> Self {
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.into(), diagram);
        self
    }

    /// Seeds `path` so that loading it reports `count` skipped unknown statements
    /// (ADR 009 v4 / RS2 — the GUI's warning path, S-014 AC4).
    #[must_use]
    pub fn with_skipped_unknowns(self, path: impl Into<PathBuf>, count: usize) -> Self {
        self.skipped_unknowns
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.into(), count);
        self
    }
}

impl DiagramStore for FakeStore {
    fn load(&self, path: &std::path::Path) -> Result<LoadedDocument, StoreError> {
        // Both lookups are infallible map reads — holding the two guards together is fine.
        let skipped = self
            .skipped_unknowns
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .copied()
            .unwrap_or(0);
        self.documents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
            .map(|diagram| LoadedDocument {
                diagram,
                skipped_unknowns: skipped,
            })
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
    /// Configured connector routes for `connector_routes` (CR-007) — empty by
    /// default, so GUI tests opt in when they exercise connector hit-testing.
    routes: Vec<(ObjectId, Vec<CellPos>)>,
}

impl FakeRenderer {
    /// Always renders `grid`, whatever the input.
    #[must_use]
    pub fn with_grid(grid: ResolvedGrid) -> Self {
        Self {
            outcome: Ok(grid),
            routes: Vec::new(),
        }
    }

    /// Sets the connector routes served by [`Renderer::connector_routes`].
    #[must_use]
    pub fn with_routes(mut self, routes: Vec<(ObjectId, Vec<CellPos>)>) -> Self {
        self.routes = routes;
        self
    }

    /// Always fails with `UnknownShapeType` — for GUI error-path tests.
    #[must_use]
    pub fn failing(type_id: impl Into<String>) -> Self {
        Self {
            outcome: Err(RenderError::UnknownShapeType {
                type_id: type_id.into(),
            }),
            routes: Vec::new(),
        }
    }
}

impl Renderer for FakeRenderer {
    fn render(&self, _diagram: &Diagram) -> Result<ResolvedGrid, RenderError> {
        self.outcome.clone()
    }

    /// The configured routes, whatever the input (CR-007).
    fn connector_routes(
        &self,
        _diagram: &Diagram,
    ) -> Result<Vec<(ObjectId, Vec<CellPos>)>, RenderError> {
        Ok(self.routes.clone())
    }

    /// The fake preview is the configured grid's text with the pattern rows
    /// prepended — GUI tests can assert on both halves.
    fn preview_pattern(&self, rows: &[String]) -> Result<ResolvedGrid, RenderError> {
        let mut all: Vec<String> = rows.to_vec();
        if let Ok(grid) = &self.outcome {
            for line in grid.to_text().lines() {
                all.push(line.to_owned());
            }
        }
        let refs: Vec<&str> = all.iter().map(String::as_str).collect();
        ResolvedGrid::from_rows(&refs).map_err(|e| RenderError::InvalidPattern {
            message: e.to_string(),
        })
    }

    /// The fake resized preview is a solid block of `#` at the requested size —
    /// GUI tests can assert on the exact dimensions; without rules it stamps the
    /// padded rows at natural size (the pipeline's rule-free behavior).
    fn preview_resized(
        &self,
        rows: &[String],
        rules: Option<&ResizeRuleSpec>,
        width: u32,
        height: u32,
    ) -> Result<ResolvedGrid, RenderError> {
        let lines: Vec<String> = match rules {
            Some(_) => vec!["#".repeat(width as usize); (height.max(1)) as usize],
            None => pad_rows(rows),
        };
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        ResolvedGrid::from_rows(&refs).map_err(|e| RenderError::InvalidPattern {
            message: e.to_string(),
        })
    }
}

/// Exporter that returns fixed media — deterministic bytes for GUI assertions.
#[derive(Debug, Default)]
pub struct FakeExporter {
    /// The colors of the most recent `to_svg`/`to_png` call (FR15 test seam: lets a
    /// GUI test assert the diagram's colors reached the exporter).
    pub last_colors: std::sync::Mutex<Option<diagram_model::color::DiagramColors>>,
}

impl Exporter for FakeExporter {
    fn to_text(&self, _grid: &ResolvedGrid) -> String {
        "+---+\n|FAKE|\n+---+".into()
    }

    fn to_svg(
        &self,
        _grid: &ResolvedGrid,
        colors: &diagram_model::color::DiagramColors,
    ) -> Result<String, ui_api::ExportError> {
        *self.last_colors.lock().expect("not poisoned") = Some(*colors);
        Ok("<svg xmlns=\"http://www.w3.org/2000/svg\">fake</svg>".into())
    }

    fn to_png(
        &self,
        _grid: &ResolvedGrid,
        colors: &diagram_model::color::DiagramColors,
    ) -> Result<Vec<u8>, ui_api::ExportError> {
        *self.last_colors.lock().expect("not poisoned") = Some(*colors);
        Ok(b"\x89PNG\r\n\x1a\nfake".to_vec())
    }
}

/// Catalog seeded with explicit read models — no registry involved. Registration
/// inserts a synthetic read model under `collection/name` (in-memory only, like the
/// rest of the fake).
#[derive(Debug, Default)]
pub struct FakeCatalog {
    entries: Mutex<BTreeMap<String, (ShapeInfo, Vec<AnchorInfo>)>>,
    /// The last `(name, text)` handed to [`import_collection`](ShapeCatalog::import_collection) —
    /// test hook for the web import flow (S-019).
    last_import: Mutex<Option<(String, String)>>,
}

impl FakeCatalog {
    /// Adds a shape with its anchors under `info.type_id`.
    #[must_use]
    pub fn add(self, info: ShapeInfo, anchors: Vec<AnchorInfo>) -> Self {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(info.type_id.clone(), (info, anchors));
        self
    }

    /// The last `(name, text)` passed to `import_collection`, if any (test hook).
    #[must_use]
    pub fn last_import(&self) -> Option<(String, String)> {
        self.last_import
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ShapeCatalog for FakeCatalog {
    fn list(&self) -> Vec<ShapeInfo> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .map(|(info, _)| info.clone())
            .collect()
    }

    fn get(&self, type_id: &str) -> Option<ShapeInfo> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(type_id)
            .map(|(info, _)| info.clone())
    }

    fn anchors(&self, type_id: &str) -> Option<Vec<AnchorInfo>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(type_id)
            .map(|(_, anchors)| anchors.clone())
    }

    fn register(&self, collection: &str, template: NewTemplate) -> Result<(), CatalogError> {
        let size = diagram_model::geometry::CellSize::new(
            template
                .rows
                .iter()
                .map(|r| r.chars().count())
                .max()
                .unwrap_or(0) as u32,
            template.rows.len() as u32,
        )
        .map_err(|e| CatalogError::Invalid {
            message: e.to_string(),
        })?;
        // S-024 AC4: rule-bearing fakes are resizable down to 2×2 (the computed
        // minimum's floor); plain customs are not (FR12).
        let (resizable, min_size) = match &template.resize_rules {
            Some(_) => (
                true,
                diagram_model::geometry::CellSize::new(2, 2).expect("2×2 is a valid cell size"),
            ),
            None => (false, size),
        };
        let info = ShapeInfo {
            type_id: format!("{collection}/{}", template.name),
            name: template.name.clone(),
            size,
            collection: Some(collection.to_owned()),
            preview: template.rows.clone(),
            resizable,
            min_size,
            resize_rules: template.resize_rules.clone(),
            regions: template.regions.clone(),
        };
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(info.type_id.clone(), (info, template.anchors.clone()));
        Ok(())
    }

    fn import_collection(&self, name: &str, text: &str) -> Result<(), CatalogError> {
        // The fake does not parse `.adc` — it records the call (the real backend's
        // validation is covered in core-services' own tests).
        *self
            .last_import
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some((name.to_owned(), text.to_owned()));
        Ok(())
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
        let seeded = store
            .load(std::path::Path::new("seeded.ad"))
            .expect("seeded");
        assert_eq!(seeded.diagram, diagram);
        assert_eq!(seeded.skipped_unknowns, 0);
        let saved = store.load(std::path::Path::new("saved.ad")).expect("saved");
        assert_eq!(saved.diagram, diagram);
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
        let exporter = FakeExporter::default();
        let grid = ResolvedGrid::from_rows(&["|"]).expect("valid grid");
        assert_eq!(exporter.to_text(&grid), "+---+\n|FAKE|\n+---+");
        let colors = diagram_model::color::DiagramColors::default();
        assert!(
            exporter
                .to_svg(&grid, &colors)
                .expect("svg")
                .contains("fake")
        );
        assert_eq!(
            &exporter.to_png(&grid, &colors).expect("png")[..4],
            b"\x89PNG"
        );

        // Catalog: list/get/anchors, including the empty-anchors and missing cases.
        let info = ShapeInfo::builtin("box", "Box", CellSize::new(9, 5).expect("valid size"));
        let anchor = AnchorInfo {
            name: "n".into(),
            position: CellPos::new(4, 0),
        };
        let catalog = FakeCatalog::default()
            .add(info.clone(), vec![anchor.clone()])
            .add(
                ShapeInfo::builtin(
                    "ellipse",
                    "Ellipse",
                    CellSize::new(7, 4).expect("valid size"),
                ),
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
