//! Renderer backend (ADR 009): a thin wrapper over `render-pipeline`'s pure
//! functions with the shape registry bound at construction.

use diagram_model::Diagram;
use render_pipeline::{RenderError, ResolvedGrid};
use shape_registry::ShapeRegistry;
use ui_api::Renderer;

/// Renders diagrams against a fixed catalog — the GUI never passes registry types
/// across the seam (C3: cross-context communication via published interfaces only).
#[derive(Debug)]
pub struct PipelineRenderer {
    registry: ShapeRegistry,
}

impl PipelineRenderer {
    /// Binds `registry` as this renderer's catalog.
    #[must_use]
    pub fn new(registry: ShapeRegistry) -> Self {
        Self { registry }
    }
}

impl Renderer for PipelineRenderer {
    fn render(&self, diagram: &Diagram) -> Result<ResolvedGrid, RenderError> {
        render_pipeline::render(diagram, &self.registry)
    }
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
