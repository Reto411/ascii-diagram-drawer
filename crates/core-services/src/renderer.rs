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
    use diagram_model::{geometry::CellPos, objects::ShapeInstance};

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
