//! render-pipeline — place → resolve → ResolvedGrid (C2, §8.4).
//!
//! Turns a [`Diagram`] plus its [`ShapeRegistry`] into the resolved character grid that
//! all export paths read. Pure and deterministic: no I/O, no randomness; identical inputs
//! produce byte-identical grids (AC1 of S-006). The only dependencies are `diagram-model`
//! and `shape-registry` — the pipeline never touches storage or the GUI.
//!
//! Stages (S-006 scope: shapes + text; connectors land in S-007):
//! 1. **place** — stamp every object at its stored coordinates into a cell grid, in
//!    document order (z-order). No layout pass.
//! 2. **resolve** — one svgbob-alphabet glyph per cell via the C1 rule table, then trim
//!    to the rendered extent and scale-expand by `Diagram::scale()`.
//!
//! The resolved grid is a derived intermediate: it is never stored as source data.

mod edges;
mod grid;
mod place;
mod resolve;

pub use grid::{GridError, ResolvedGrid, expand};

use diagram_model::Diagram;
use shape_registry::ShapeRegistry;

/// Errors the render pipeline can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A shape instance references a type id that is not in the registry.
    UnknownShapeType {
        /// The unknown `type_id` as stored on the instance.
        type_id: String,
    },
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownShapeType { type_id } => {
                write!(f, "unknown shape type id `{type_id}`")
            }
        }
    }
}

impl std::error::Error for RenderError {}

/// Renders a diagram to its resolved grid: place → resolve → scale-expand (C2).
///
/// Pure and deterministic — the same inputs always yield byte-identical output.
/// Connectors are ignored in S-006 (they are rendered from S-007 onward).
pub fn render(d: &Diagram, registry: &ShapeRegistry) -> Result<ResolvedGrid, RenderError> {
    let placed = place::place(d, registry)?;
    let resolved = resolve::resolve(&placed);
    Ok(expand(&resolved, d.scale()))
}

/// Whether `c` is a structural line glyph of the svgbob alphabet: it connects to its
/// neighbors and fills its whole scale block when the grid is expanded (FR8). The quote
/// character is content, not structure.
pub const fn is_line_glyph(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '|' | '.' | ',' | ':' | ';' | '/' | '\\' | '_' | '`' | '\'' | '(' | ')'
    )
}

/// Whether `c` belongs to svgbob's ASCII input alphabet (ADR 001 v2): the structural line
/// glyphs plus double quotes.
pub const fn is_svgbob_glyph(c: char) -> bool {
    c == '"' || is_line_glyph(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::{ObjectId, ShapeInstance};

    #[test]
    fn render_is_pure_and_deterministic() {
        let mut d1 = Diagram::new();
        d1.place_shape(
            ShapeInstance::new(
                ObjectId::generated(0),
                "box",
                "hi",
                CellPos::new(2, 1),
                CellSize::new(7, 3).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
        .unwrap();
        let d2 = d1.clone();
        let registry = ShapeRegistry::builtin();
        assert_eq!(
            render(&d1, &registry).unwrap().to_text(),
            render(&d2, &registry).unwrap().to_text()
        );
    }

    #[test]
    fn render_empty_diagram_is_empty() {
        let g = render(&Diagram::new(), &ShapeRegistry::builtin()).unwrap();
        assert!(g.is_empty());
    }

    #[test]
    fn scale_expands_the_whole_grid() {
        let mut d = Diagram::new();
        d.place_shape(
            ShapeInstance::new(
                ObjectId::generated(0),
                "box",
                "",
                CellPos::new(0, 0),
                CellSize::new(5, 3).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
        .unwrap();
        d.set_scale(2).unwrap();
        let g = render(&d, &ShapeRegistry::builtin()).unwrap();
        assert_eq!(g.width(), 10);
        assert_eq!(g.height(), 6);
    }

    #[test]
    fn svgbob_glyph_set_is_exact() {
        for c in "+-.|,:;/\\_`'()\"".chars() {
            assert!(is_svgbob_glyph(c), "`{c}` must be in the alphabet");
        }
        assert!(!is_svgbob_glyph('a'));
        assert!(!is_svgbob_glyph(' '));
        assert!(!is_line_glyph('"'), "quotes are content, not structure");
    }
}
