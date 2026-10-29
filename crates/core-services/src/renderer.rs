//! Renderer backend (ADR 009): a thin wrapper over `render-pipeline`'s pure
//! functions with the shape registry bound at construction.

use diagram_model::{Diagram, geometry::CellPos, objects::ShapeInstance};
use render_pipeline::{RenderError, ResolvedGrid};
use shape_registry::{AnchorSet, Collection, GeometryKind, ShapeRegistry, ShapeType, TemplateGrid};
use ui_api::Renderer;

use crate::registry_state::{SharedRegistry, pad_rows};

/// Renders diagrams against the shared catalog — the GUI never passes registry
/// types across the seam (C3: cross-context communication via published interfaces
/// only). The registry is [`SharedRegistry`], so templates registered through the
/// catalog service are rendered without re-composition (S-017).
#[derive(Debug)]
pub struct PipelineRenderer {
    state: SharedRegistry,
}

impl PipelineRenderer {
    /// Binds `registry` as this renderer's catalog (standalone state — no sharing
    /// with a catalog service; for tests and simple compositions).
    #[must_use]
    pub fn new(registry: ShapeRegistry) -> Self {
        Self::with_shared(SharedRegistry::standalone(registry))
    }

    /// Binds the shared state composed with a [`RegistryCatalog`](crate::RegistryCatalog)
    /// — registration through the catalog is visible to this renderer (S-017).
    #[must_use]
    pub fn with_shared(state: SharedRegistry) -> Self {
        Self { state }
    }
}

impl Renderer for PipelineRenderer {
    fn render(&self, diagram: &Diagram) -> Result<ResolvedGrid, RenderError> {
        render_pipeline::render(diagram, &self.state.snapshot())
    }

    fn preview_pattern(&self, rows: &[String]) -> Result<ResolvedGrid, RenderError> {
        let grid = pattern_grid(rows)?;
        // A throwaway registry holding just the synthetic template: the pattern is
        // placed at the origin of an otherwise empty diagram and resolved by the same
        // place/resolve stages a saved template would use (S-017 AC1).
        let type_id = format!("{PREVIEW_COLLECTION}/{PREVIEW_TEMPLATE}");
        let size = diagram_model::geometry::CellSize::new(grid.width(), grid.height())
            .expect("grid dimensions are valid cell sizes");
        let mut registry = ShapeRegistry::builtin();
        registry.add_collection(Collection {
            name: PREVIEW_COLLECTION.to_owned(),
            types: vec![ShapeType {
                type_id: type_id.clone(),
                name: PREVIEW_TEMPLATE.to_owned(),
                glyph_pattern: grid,
                anchor_set: AnchorSet::new(),
                geometry: GeometryKind::Pattern,
                resize_rules: None,
            }],
        });

        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        let instance = ShapeInstance::new(id, &type_id, "", CellPos::origin(), size)
            .expect("valid instance: pattern is a well-formed grid at the origin");
        diagram
            .place_shape(instance)
            .expect("single shape places cleanly");
        render_pipeline::render(&diagram, &registry)
    }
}

/// The reserved collection name for preview patterns — never a real collection
/// (the synthetic registry is throwaway, so even a collision could not leak).
const PREVIEW_COLLECTION: &str = "zz_preview";
/// The reserved template name inside the preview collection.
const PREVIEW_TEMPLATE: &str = "zz_pattern";

/// Builds a [`TemplateGrid`] from raw preview rows, mapping grid failures to the
/// seam's typed error (the editor pre-validates, but the seam must not trust it).
fn pattern_grid(rows: &[String]) -> Result<TemplateGrid, RenderError> {
    let padded = pad_rows(rows);
    if padded.iter().all(|r| r.chars().all(|c| c == ' ')) {
        return Err(RenderError::InvalidPattern {
            message: "the pattern is empty".to_owned(),
        });
    }
    let refs: Vec<&str> = padded.iter().map(String::as_str).collect();
    TemplateGrid::from_rows(&refs).map_err(|e| RenderError::InvalidPattern {
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryCatalog;
    use diagram_model::{geometry::CellPos, objects::ShapeInstance};
    use ui_api::ShapeCatalog;

    /// AC2 — the backend is a faithful pass-through: a real model renders to the same
    /// grid as calling the pipeline directly.
    #[test]
    fn renders_exactly_like_the_pipeline() {
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        let instance = ShapeInstance::new(
            id,
            "box",
            "",
            CellPos::origin(),
            diagram_model::geometry::CellSize::new(9, 5).expect("valid size"),
        )
        .expect("valid instance");
        diagram.place_shape(instance).expect("valid placement");

        let renderer = PipelineRenderer::new(ShapeRegistry::builtin());
        assert_eq!(
            renderer.render(&diagram).expect("renders"),
            render_pipeline::render(&diagram, &ShapeRegistry::builtin()).expect("renders")
        );
    }

    /// S-016 AC3 — a placed custom template is stamped verbatim: every glyph row of
    /// its pattern appears in the rendered grid (the opaque-stamp rule, S-006), and
    /// the catalog serves its anchors for connectors.
    #[test]
    fn custom_templates_stamp_verbatim_and_keep_live_anchors() {
        use shape_registry::{AnchorSet, Collection, ShapeType, TemplateGrid};

        let mut registry = ShapeRegistry::builtin();
        let mut anchors = AnchorSet::new();
        anchors
            .add("e", shape_registry::CellPos { x: 2, y: 1 })
            .expect("valid anchor");
        let grid = TemplateGrid::from_rows(&["#.#", "###", ".#."]).expect("valid grid");
        registry.add_collection(Collection {
            name: "col".into(),
            types: vec![ShapeType {
                type_id: "col/star".into(),
                name: "star".into(),
                glyph_pattern: grid.clone(),
                anchor_set: anchors,
                geometry: shape_registry::GeometryKind::Pattern,
                resize_rules: None,
            }],
        });

        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        let instance = ShapeInstance::new(
            id,
            "col/star",
            "",
            CellPos::origin(),
            diagram_model::geometry::CellSize::new(3, 3).expect("valid size"),
        )
        .expect("valid instance");
        diagram.place_shape(instance).expect("valid placement");

        let renderer = PipelineRenderer::new(registry.clone());
        let resolved = renderer.render(&diagram).expect("renders");
        let text = exporters::to_text(&resolved);
        for row in 0..grid.height() {
            let expected = grid.row(row).expect("in-bounds row");
            assert!(
                text.lines().any(|line| line.contains(&expected)),
                "pattern row {expected:?} must appear verbatim in:\n{text}"
            );
        }

        // Anchors stay live for connectors (S-013): the catalog resolves them.
        let catalog = RegistryCatalog::new(registry);
        let anchors = catalog.anchors("col/star").expect("anchor set");
        assert_eq!(anchors.len(), 1);
        assert_eq!((anchors[0].position.x, anchors[0].position.y), (2, 1));
    }

    /// S-017 AC1 — the preview equals the render output: a pattern previewed through
    /// `preview_pattern` renders byte-identically to the same pattern saved as a real
    /// template and placed in a diagram.
    #[test]
    fn preview_equals_render_output_for_the_same_pattern() {
        let rows: Vec<String> = vec!["#.#".into(), "###".into(), ".#.".into()];

        // Left: the live preview of the raw pattern.
        let renderer = PipelineRenderer::new(ShapeRegistry::builtin());
        let preview = renderer.preview_pattern(&rows).expect("previews");

        // Right: the same pattern saved as a template and rendered from a diagram.
        use shape_registry::{AnchorSet, Collection, ShapeType};
        let mut registry = ShapeRegistry::builtin();
        let mut anchors = AnchorSet::new();
        anchors
            .add("n", shape_registry::CellPos { x: 1, y: 0 })
            .unwrap();
        registry.add_collection(Collection {
            name: "col".into(),
            types: vec![ShapeType {
                type_id: "col/pat".into(),
                name: "pat".into(),
                glyph_pattern: TemplateGrid::from_rows(&["#.#", "###", ".#."]).unwrap(),
                anchor_set: anchors,
                geometry: shape_registry::GeometryKind::Pattern,
                resize_rules: None,
            }],
        });
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        let instance = ShapeInstance::new(
            id,
            "col/pat",
            "",
            CellPos::origin(),
            diagram_model::geometry::CellSize::new(3, 3).expect("valid size"),
        )
        .expect("valid instance");
        diagram.place_shape(instance).expect("places");
        let rendered = PipelineRenderer::new(registry)
            .render(&diagram)
            .expect("renders");

        assert_eq!(preview.to_text(), rendered.to_text());
    }

    /// S-017 AC1 — ragged preview rows are right-padded (parser rule), empty and
    /// non-printable patterns are typed errors, not panics.
    #[test]
    fn preview_rejects_empty_and_invalid_patterns() {
        let renderer = PipelineRenderer::new(ShapeRegistry::builtin());
        assert!(
            renderer
                .preview_pattern(&["ab".into(), "abcd".into()])
                .is_ok()
        ); // ragged rows are right-padded
        assert!(matches!(
            renderer.preview_pattern(&[]),
            Err(RenderError::InvalidPattern { .. })
        ));
        assert!(matches!(
            renderer.preview_pattern(&["   ".into()]),
            Err(RenderError::InvalidPattern { .. })
        ));
        assert!(matches!(
            renderer.preview_pattern(&["é".into()]),
            Err(RenderError::InvalidPattern { .. })
        ));
    }

    /// Unknown type ids surface as the typed `RenderError` — no panics at the seam.
    #[test]
    fn unknown_shape_types_are_typed_errors() {
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        let instance = ShapeInstance::new(
            id,
            "nope",
            "",
            CellPos::origin(),
            diagram_model::geometry::CellSize::new(3, 2).expect("valid size"),
        )
        .expect("valid instance");
        diagram.place_shape(instance).expect("valid placement");

        let renderer = PipelineRenderer::new(ShapeRegistry::builtin());
        assert!(matches!(
            renderer.render(&diagram),
            Err(RenderError::UnknownShapeType { .. })
        ));
    }
}
