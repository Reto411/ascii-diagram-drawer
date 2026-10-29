//! The composition root (ADR 009): one `Services` aggregate holds all four seam
//! services as `Arc<dyn …>` — exactly what the eframe app struct will carry.

use std::path::Path;
use std::sync::Arc;

use shape_registry::ShapeRegistry;
use ui_api::{DiagramStore, Exporter, Renderer, ShapeCatalog};

use crate::catalog::RegistryCatalog;
use crate::collections::{CollectionLoadWarning, load_from_dir};
use crate::exporter::GridExporter;
use crate::registry_state::SharedRegistry;
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
    /// Shape type listing/lookup + template registration (FR3/FR11, S-017).
    pub catalog: Arc<dyn ShapeCatalog>,
}

impl Services {
    /// Native composition: filesystem store + built-in catalog + full exporters.
    /// No collections directory — template registration reports a clear IO error
    /// (use [`native_with_collections`](Self::native_with_collections) to enable it).
    #[must_use]
    pub fn native() -> Self {
        let state = SharedRegistry::standalone(ShapeRegistry::builtin());
        Self {
            store: Arc::new(FsStore),
            renderer: Arc::new(PipelineRenderer::with_shared(state.clone())),
            exporter: Arc::new(GridExporter),
            catalog: Arc::new(RegistryCatalog::with_shared(state)),
        }
    }

    /// Native composition with collections discovered from `dir` (S-016 AC2): every
    /// `*.adc` file loads into the catalog; invalid files are skipped and reported in
    /// the returned warnings — startup always succeeds. `None` = built-ins only.
    #[must_use]
    pub fn native_with_collections(dir: Option<&Path>) -> (Self, Vec<CollectionLoadWarning>) {
        let mut registry = ShapeRegistry::builtin();
        let warnings = match dir {
            Some(d) => load_from_dir(&mut registry, d),
            None => Vec::new(),
        };
        // One shared state for renderer + catalog: templates registered through the
        // catalog (S-017) render immediately; `dir` is where they persist.
        let state = match dir {
            Some(d) => SharedRegistry::with_collections_dir(registry, d.to_path_buf()),
            None => SharedRegistry::standalone(registry),
        };
        (
            Self {
                store: Arc::new(FsStore),
                renderer: Arc::new(PipelineRenderer::with_shared(state.clone())),
                exporter: Arc::new(GridExporter),
                catalog: Arc::new(RegistryCatalog::with_shared(state)),
            },
            warnings,
        )
    }

    /// Web (wasm) composition: in-memory store; everything else identical — the seam
    /// does not change shape per target, only the store backend. No collections
    /// directory (no filesystem on wasm) → registration reports a clear IO error.
    #[must_use]
    pub fn web() -> Self {
        let state = SharedRegistry::standalone(ShapeRegistry::builtin());
        Self {
            store: Arc::new(MemoryStore::default()),
            renderer: Arc::new(PipelineRenderer::with_shared(state.clone())),
            exporter: Arc::new(GridExporter),
            catalog: Arc::new(RegistryCatalog::with_shared(state)),
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
                ShapeInfo::builtin(
                    "box",
                    "Box",
                    diagram_model::geometry::CellSize::new(9, 5).expect("valid size"),
                ),
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

    /// S-016 AC2 — the composition root discovers collections: valid files reach the
    /// catalog (with group + preview fields), invalid ones become warnings, and
    /// startup always succeeds.
    #[test]
    fn native_with_collections_loads_valid_and_warns_on_invalid() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("stars.adc"),
            "adc 1.0\ntemplate star 3 x 3\n###\n#.#\n###\nanchor n at 1 0\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("broken.adc"),
            "adc 1.0\ntemplate x 2 x 2\na\nb\nc\n",
        )
        .unwrap();

        let (services, warnings) = Services::native_with_collections(Some(dir.path()));

        // The valid collection is usable through the seam…
        let star = services
            .catalog
            .get("stars/star")
            .expect("collection shape");
        assert_eq!(star.collection.as_deref(), Some("stars"));
        assert_eq!(star.preview, vec!["###", "#.#", "###"]);

        // …the broken one is a warning with line/column, not an error…
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].file, "broken.adc");
        assert!(
            warnings[0].reason.starts_with("line "),
            "{:?}",
            warnings[0].reason
        );

        // …and built-ins survived (startup always succeeds).
        assert!(services.catalog.get("box").is_some());
        let list = services.catalog.list();
        assert!(
            list.iter()
                .all(|s| s.collection.as_deref() != Some("broken"))
        );
    }

    /// S-017 AC3 — registration through the catalog is visible to the renderer in
    /// the same composition: one shared registry behind both seam services.
    #[test]
    fn registered_templates_render_immediately() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (services, _) = Services::native_with_collections(Some(dir.path()));

        services
            .catalog
            .register(
                "stars",
                ui_api::NewTemplate {
                    name: "star".into(),
                    rows: vec!["#.#".into(), "###".into(), "#.#".into()],
                    anchors: vec![ui_api::AnchorInfo {
                        name: "n".into(),
                        position: diagram_model::geometry::CellPos { x: 1, y: 0 },
                    }],
                },
            )
            .expect("valid template");

        // The sidebar lists it…
        let star = services.catalog.get("stars/star").expect("listed");
        assert_eq!(star.size.width, 3);

        // …and a diagram using it renders through the same composition.
        let mut d = diagram_model::Diagram::new();
        let id = d.new_object_id();
        let instance = ShapeInstance::new(
            id,
            "stars/star",
            "",
            CellPos::origin(),
            diagram_model::geometry::CellSize::new(3, 3).expect("valid size"),
        )
        .expect("valid instance");
        d.place_shape(instance).expect("places");
        let text = services
            .exporter
            .to_text(&services.renderer.render(&d).expect("renders"));
        assert!(text.contains("###"), "pattern must be stamped in:\n{text}");

        // The file was persisted next to the discovered collections.
        assert!(dir.path().join("stars.adc").exists());
    }

    /// S-016 AC2 — `None` keeps the built-in-only composition (no discovery).
    #[test]
    fn native_with_no_collections_dir_is_builtins_only() {
        let (services, warnings) = Services::native_with_collections(None);
        assert!(warnings.is_empty());
        assert!(
            services
                .catalog
                .list()
                .iter()
                .all(|s| s.collection.is_none())
        );
    }
}
