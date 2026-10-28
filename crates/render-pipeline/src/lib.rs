//! render-pipeline — place → resolve → ResolvedGrid (C2, §8.4).
//!
//! Turns a [`Diagram`] plus its [`ShapeRegistry`] into the resolved character grid that
//! all export paths read. Pure and deterministic: no I/O, no randomness; identical inputs
//! produce byte-identical grids (AC1 of S-006). The only dependencies are `diagram-model`
//! and `shape-registry` — the pipeline never touches storage or the GUI.
//!
//! Stages:
//! 1. **place** — stamp every object at its stored coordinates into a cell grid, in
//!    document order (z-order). No layout pass. Connectors re-resolve their anchors and
//!    stamp routed edges + an arrowhead (ADR 013).
//! 2. **resolve** — one svgbob-alphabet glyph per cell via the C1 rule table, then trim
//!    to the rendered extent and scale-expand by `Diagram::scale()`.
//!
//! The resolved grid is a derived intermediate: it is never stored as source data.

mod connectors;
mod edges;
mod grid;
mod place;
mod resolve;

pub use grid::{GridError, ResolvedGrid, expand};

use diagram_model::Diagram;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::ObjectId;
use shape_registry::ShapeRegistry;

/// Errors the render pipeline can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A shape instance references a type id that is not in the registry.
    UnknownShapeType {
        /// The unknown `type_id` as stored on the instance.
        type_id: String,
    },
    /// A connector endpoint names an anchor the shape's type does not declare (ADR 013).
    UnknownAnchor {
        /// The object whose anchor set was looked up.
        object: ObjectId,
        /// The missing anchor name.
        anchor: String,
    },
    /// A connector endpoint's anchor is declared outside its type's template extent and
    /// cannot resolve inside the shape's bounds (§8.4.1 render-time invariant, ADR 013).
    AnchorOutsideExtent {
        /// The object whose anchor set was looked up.
        object: ObjectId,
        /// The offending anchor name.
        anchor: String,
        /// The declared (template-space) anchor position.
        pos: CellPos,
        /// The template extent the position falls outside of.
        extent: CellSize,
    },
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownShapeType { type_id } => {
                write!(f, "unknown shape type id `{type_id}`")
            }
            Self::UnknownAnchor { object, anchor } => {
                write!(f, "object `{object}` has no anchor named `{anchor}`")
            }
            Self::AnchorOutsideExtent {
                object,
                anchor,
                pos,
                extent,
            } => write!(
                f,
                "anchor `{anchor}` of object `{object}` is at ({}, {}), outside the {}x{} template extent",
                pos.x, pos.y, extent.width, extent.height
            ),
        }
    }
}

impl std::error::Error for RenderError {}

/// Renders a diagram to its resolved grid: place → resolve → scale-expand (C2).
///
/// Pure and deterministic — the same inputs always yield byte-identical output.
/// Connectors are routed and arrowheaded per ADR 013; their endpoints are re-resolved
/// from anchor references on every call, so moved/resized shapes carry their connectors.
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

/// Whether `c` is an arrowhead glyph produced by connector routing (ADR 013): solid
/// `< > ^ v`, stamped as content at the target end of a route.
pub const fn is_arrowhead(c: char) -> bool {
    matches!(c, '<' | '>' | '^' | 'v')
}

/// Whether `c` belongs to svgbob's ASCII input alphabet (ADR 001 v2): the structural line
/// glyphs, double quotes, and the arrow vocabulary `< > ^ v V`. (The pipeline only ever
/// emits the lowercase arrows as connector heads; uppercase `V` appears in user text,
/// where it must still count for quote-escaping per C1 F4.)
pub const fn is_svgbob_glyph(c: char) -> bool {
    c == '"' || c == 'V' || is_line_glyph(c) || is_arrowhead(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::anchors::AnchorRef;
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::{ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance};

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
    fn render_error_display_names_what_is_wrong() {
        assert_eq!(
            RenderError::UnknownShapeType {
                type_id: "nope".into()
            }
            .to_string(),
            "unknown shape type id `nope`"
        );
        assert_eq!(
            RenderError::UnknownAnchor {
                object: ObjectId::generated(7),
                anchor: "e".into(),
            }
            .to_string(),
            "object `obj-7` has no anchor named `e`"
        );
        assert_eq!(
            RenderError::AnchorOutsideExtent {
                object: ObjectId::generated(8),
                anchor: "out".into(),
                pos: CellPos::new(5, 0),
                extent: CellSize::new(4, 2).expect("non-zero test size"),
            }
            .to_string(),
            "anchor `out` of object `obj-8` is at (5, 0), outside the 4x2 template extent"
        );
    }

    #[test]
    fn svgbob_glyph_set_is_exact() {
        for c in "+-.|,:;/\\_`'()\"<^>vV".chars() {
            assert!(is_svgbob_glyph(c), "`{c}` must be in the alphabet");
        }
        assert!(!is_svgbob_glyph('a'));
        assert!(!is_svgbob_glyph(' '));
        assert!(!is_line_glyph('"'), "quotes are content, not structure");
        assert!(!is_line_glyph('>'), "arrowheads are content, not structure");
    }

    /// A box pair with a solid connector between them (ids 0/1, connector id 2).
    fn connected_boxes() -> Diagram {
        let mut d = Diagram::new();
        d.place_shape(
            ShapeInstance::new(
                ObjectId::generated(0),
                "box",
                "A",
                CellPos::new(0, 0),
                CellSize::new(9, 5).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
        .unwrap();
        d.place_shape(
            ShapeInstance::new(
                ObjectId::generated(1),
                "box",
                "B",
                CellPos::new(14, 0),
                CellSize::new(9, 5).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
        .unwrap();
        d.connect(ConnectorInstance::new(
            ObjectId::generated(2),
            AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        ))
        .unwrap();
        d
    }

    #[test]
    fn connectors_are_routed_and_arrowheaded() {
        let g = render(&connected_boxes(), &ShapeRegistry::builtin()).unwrap();
        // A.e -> B.w: straight route with a T-junction at each anchor and '>' before B.
        assert_eq!(
            g.to_text().lines().nth(2).expect("third row"),
            "|   A   +---->+   B   |"
        );
    }

    /// The same final state built two ways: box B placed at `pos` from the start, or
    /// placed at (14, 0) and then moved there. Both must render byte-identically —
    /// connectors store no geometry, so the route re-resolves on every render (AC3).
    fn boxes_with_b_at(pos: CellPos) -> Diagram {
        let mut d = connected_boxes();
        d.move_object(&ObjectId::generated(1), pos).unwrap();
        d
    }

    #[test]
    fn connector_follows_moves_and_resizes() {
        let registry = ShapeRegistry::builtin();

        // Move: build the final state directly (B placed at (20, 3)) vs. place-then-move.
        let direct_move = {
            let mut d = Diagram::new();
            d.place_shape(
                ShapeInstance::new(
                    ObjectId::generated(0),
                    "box",
                    "A",
                    CellPos::new(0, 0),
                    CellSize::new(9, 5).expect("non-zero test size"),
                )
                .expect("valid shape"),
            )
            .unwrap();
            d.place_shape(
                ShapeInstance::new(
                    ObjectId::generated(1),
                    "box",
                    "B",
                    CellPos::new(20, 3),
                    CellSize::new(9, 5).expect("non-zero test size"),
                )
                .expect("valid shape"),
            )
            .unwrap();
            d.connect(ConnectorInstance::new(
                ObjectId::generated(2),
                AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
                AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
                ConnectorStyle::Solid,
            ))
            .unwrap();
            d
        };
        let via_move = boxes_with_b_at(CellPos::new(20, 3));
        assert_eq!(direct_move, via_move, "same final model either way");
        assert_eq!(
            render(&direct_move, &registry).unwrap().to_text(),
            render(&via_move, &registry).unwrap().to_text()
        );

        // Resize: B grown to 12x7 in place — anchors re-map proportionally.
        let direct_resize = {
            let mut d = Diagram::new();
            d.place_shape(
                ShapeInstance::new(
                    ObjectId::generated(0),
                    "box",
                    "A",
                    CellPos::new(0, 0),
                    CellSize::new(9, 5).expect("non-zero test size"),
                )
                .expect("valid shape"),
            )
            .unwrap();
            d.place_shape(
                ShapeInstance::new(
                    ObjectId::generated(1),
                    "box",
                    "B",
                    CellPos::new(14, 0),
                    CellSize::new(12, 7).expect("non-zero test size"),
                )
                .expect("valid shape"),
            )
            .unwrap();
            d.connect(ConnectorInstance::new(
                ObjectId::generated(2),
                AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
                AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
                ConnectorStyle::Solid,
            ))
            .unwrap();
            d
        };
        let via_resize = {
            let mut d = connected_boxes();
            d.resize(
                &ObjectId::generated(1),
                CellSize::new(12, 7).expect("non-zero test size"),
            )
            .unwrap();
            d
        };
        assert_eq!(direct_resize, via_resize, "same final model either way");
        assert_eq!(
            render(&direct_resize, &registry).unwrap().to_text(),
            render(&via_resize, &registry).unwrap().to_text()
        );
    }
}
