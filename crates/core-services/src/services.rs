//! The composition root (ADR 009): one `Services` aggregate holds all four seam
//! services as `Arc<dyn …>` — exactly what the eframe app struct will carry.

use std::sync::Arc;

use shape_registry::ShapeRegistry;
use ui_api::{DiagramStore, Exporter, Renderer, ShapeCatalog};

use crate::catalog::RegistryCatalog;
use crate::exporter::GridExporter;
use crate::renderer::PipelineRenderer;
use crate::store::{FsStore, MemoryStore};

/// All application services behind the ADR 009 seam. The GUI depends only on this
/// type (plus `ui-api` traits and `diagram-model` value types) — never on a backend.
#[derive(Clone)]
pub struct Services {
    /// `.ad` load/save (FR4). Filesystem on native, in-memory on wasm.
    pub store: Arc<dyn DiagramStore>,
    /// Model → resolved grid (bound to the catalog at construction).
    pub renderer: Arc<dyn Renderer>,
    /// Grid → text/SVG/PNG (FR2/FR5); PNG degrades to `Unsupported` without a backend.
    pub exporter: Arc<dyn Exporter>,
    /// Shape type listing/lookup (FR3; registration is v1+).
    pub catalog: Arc<dyn ShapeCatalog>,
}

impl Services {
    /// Native composition: filesystem store + built-in catalog + full exporters.
    #[must_use]
    pub fn native() -> Self {
        let registry = ShapeRegistry::builtin();
        Self {
            store: Arc::new(FsStore),
            renderer: Arc::new(PipelineRenderer::new(registry.clone())),
            exporter: Arc::new(GridExporter),
            catalog: Arc::new(RegistryCatalog::new(registry)),
        }
    }

    /// Web (wasm) composition: in-memory store; everything else identical — the seam
    /// does not change shape per target, only the store backend.
    #[must_use]
    pub fn web() -> Self {
        let registry = ShapeRegistry::builtin();
        Self {
            store: Arc::new(MemoryStore::default()),
            renderer: Arc::new(PipelineRenderer::new(registry.clone())),
            exporter: Arc::new(GridExporter),
            catalog: Arc::new(RegistryCatalog::new(registry)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{FakeCatalog, FakeExporter, FakeRenderer, FakeStore};
    use diagram_model::{geometry::CellPos, objects::ShapeInstance};
    use render_pipeline::ResolvedGrid;
    use ui_api::ShapeInfo;

    fn one_box() -> diagram_model::Diagram {
        let mut d = diagram_model::Diagram::new();
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

    /// AC2 — both compositions are fully wired: a real model renders and exports
    /// through the trait objects, and the catalog answers.
    #[test]
    fn both_compositions_serve_a_full_flow() {
        for services in [Services::native(), Services::web()] {
            let diagram = one_box();
            let grid = services.renderer.render(&diagram).expect("renders");
            assert!(!services.exporter.to_text(&grid).is_empty());
            assert!(
                services
                    .exporter
                    .to_svg(&grid)
                    .expect("svg")
                    .starts_with("<svg")
            );
            assert_eq!(
                &services.exporter.to_png(&grid).expect("png")[..8],
                b"\x89PNG\r\n\x1a\n"
            );
            assert!(services.catalog.get("box").is_some());
        }
    }

    /// AC3 — the T7 pattern: a fake `Services` is usable exactly like production,
    /// through trait objects only. This is the example future GUI tests build on.
    #[test]
    fn fakes_drive_the_same_trait_objects() {
        let services = Services {
            store: Arc::new(FakeStore::default().with_document("a.ad", one_box())),
            renderer: Arc::new(FakeRenderer::with_grid(
                ResolvedGrid::from_rows(&["+---+"]).expect("valid grid"),
            )),
            exporter: Arc::new(FakeExporter),
            catalog: Arc::new(FakeCatalog::default().add(
                ShapeInfo {
                    type_id: "box".into(),
                    name: "Box".into(),
                    size: diagram_model::geometry::CellSize::new(9, 5).expect("valid size"),
                },
                Vec::new(),
            )),
        };

        // The GUI's whole service flow — no backend type in sight.
        let loaded = services
            .store
            .load(std::path::Path::new("a.ad"))
            .expect("seeded");
        let diagram = loaded.diagram;
        assert_eq!(diagram, one_box());
        let grid = services.renderer.render(&diagram).expect("fake render");
        assert_eq!(grid.to_text(), "+---+");
        assert!(services.exporter.to_text(&grid).contains("FAKE"));
        assert!(services.catalog.get("box").is_some());

        // And the error paths are fakeable too.
        let failing = Services {
            renderer: Arc::new(FakeRenderer::failing("nope")),
            ..Services::web()
        };
        assert!(matches!(
            failing.renderer.render(&diagram),
            Err(render_pipeline::RenderError::UnknownShapeType { .. })
        ));
    }
}
