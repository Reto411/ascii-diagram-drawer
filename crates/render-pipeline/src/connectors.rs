//! Connector placement (ADR 013): resolve both endpoint anchors to cells, route a
//! straight or single-corner L between them, and stamp the route as structural edges
//! with an arrowhead at the target end.
//!
//! A connector stores no geometry — its endpoints are anchor *references* re-resolved on
//! every render (FR9), so moving or resizing a shape moves its connectors for free and
//! rendering `{model, then move_object}` is byte-identical to rendering the moved model.
//! Route cells are stamped in document order like any other object: edges OR-merge with
//! border/art neighbors (C1 F2), structural stamps claim the top so later objects can
//! still cover this connector (F1), and the arrowhead is content stamped last — it never
//! changes edge resolution of neighboring cells.

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{ConnectorInstance, Object};
use shape_registry::{GeometryKind, ShapeRegistry};

use crate::RenderError;
use crate::edges::EdgeSet;
use crate::place::{PlacedGrid, lookup_shape};

/// One side of a shape's extent — used to prefer the natural departure/arrival direction
/// when choosing an L-corner (ADR 013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

/// The side a resolved anchor cell lies on: `Some` only when it is strictly on one side,
/// `None` for corner cells (two sides) and interior cells (no side). Degenerate extents
/// (width or height 1) make two sides coincide, which also yields `None`.
fn anchor_side(px: u32, py: u32, width: u32, height: u32, pos: CellPos) -> Option<Side> {
    let mut found = None;
    for (on_side, side) in [
        (pos.y == py, Side::Top),
        (pos.y == py + height - 1, Side::Bottom),
        (pos.x == px, Side::Left),
        (pos.x == px + width - 1, Side::Right),
    ] {
        if on_side && found.is_some() {
            return None; // corner cell or degenerate extent: no single side
        }
        if on_side {
            found = Some(side);
        }
    }
    found
}

/// A resolved connector endpoint: the anchor's absolute cell plus the shape side it lies
/// on (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Endpoint {
    cell: CellPos,
    side: Option<Side>,
}

/// Resolves one endpoint reference to an absolute cell (ADR 013): `Rect` and `Ellipse`
/// geometry map template coordinates proportionally onto the instance extent; `Pattern`
/// geometry is a plain offset at natural size.
fn resolve_endpoint(
    diagram: &Diagram,
    registry: &ShapeRegistry,
    reference: &AnchorRef,
) -> Result<Endpoint, RenderError> {
    // The model guarantees endpoints exist and are shapes (Diagram::connect validates);
    // the pipeline is the last line of defense and fails loudly if that breaks.
    let object = diagram.get(reference.object()).expect(
        "connector endpoint must reference an existing object (Diagram::connect enforces this)",
    );
    let Object::Shape(instance) = object else {
        panic!("connector endpoint must be a shape (Diagram::connect enforces this)");
    };
    let shape = lookup_shape(instance, registry)?;

    let Some(template_pos) = shape.anchor_set.get(&reference.anchor) else {
        return Err(RenderError::UnknownAnchor {
            object: reference.object().clone(),
            anchor: reference.anchor.clone(),
        });
    };

    // §8.4.1 render-time invariant: an anchor must sit inside its type's template extent,
    // otherwise it cannot resolve inside the shape's bounds.
    let (tw, th) = (shape.glyph_pattern.width(), shape.glyph_pattern.height());
    if template_pos.x >= tw || template_pos.y >= th {
        return Err(RenderError::AnchorOutsideExtent {
            object: reference.object().clone(),
            anchor: reference.anchor.clone(),
            pos: CellPos::new(template_pos.x, template_pos.y),
            extent: CellSize::new(tw, th).expect("template extents are non-zero"),
        });
    }

    let (cell, width, height) = match shape.geometry {
        GeometryKind::Rect | GeometryKind::Ellipse => {
            let w = instance.size.width;
            let h = instance.size.height;
            (
                CellPos::new(
                    instance.pos.x + crate::place::map_proportional(template_pos.x, tw, w),
                    instance.pos.y + crate::place::map_proportional(template_pos.y, th, h),
                ),
                w,
                h,
            )
        }
        GeometryKind::Pattern => (
            CellPos::new(
                instance.pos.x + template_pos.x,
                instance.pos.y + template_pos.y,
            ),
            tw,
            th,
        ),
    };

    Ok(Endpoint {
        cell,
        side: anchor_side(instance.pos.x, instance.pos.y, width, height, cell),
    })
}

/// The route cells between two endpoints (ADR 013): straight when they share a row or
/// column, otherwise a single-corner L — the target's side preference wins, then the
/// source's, then vertical-first. Both endpoint cells are included; coincident endpoints
/// produce an empty route (nothing is drawn).
fn route(
    from: CellPos,
    to: CellPos,
    from_side: Option<Side>,
    to_side: Option<Side>,
) -> Vec<CellPos> {
    if from == to {
        return Vec::new();
    }
    if from.x != to.x && from.y != to.y {
        let corner = match to_side {
            Some(Side::Left | Side::Right) => CellPos::new(from.x, to.y), // arrive horizontally
            Some(Side::Top | Side::Bottom) => CellPos::new(to.x, from.y), // arrive vertically
            None => match from_side {
                Some(Side::Left | Side::Right) => CellPos::new(to.x, from.y), // depart horizontally
                _ => CellPos::new(from.x, to.y), // default: vertical first
            },
        };
        let mut cells = straight_line(from, corner);
        cells.extend(straight_line(corner, to).into_iter().skip(1)); // the corner is shared
        return cells;
    }
    straight_line(from, to)
}

/// Inclusive cells of a horizontal or vertical segment.
fn straight_line(a: CellPos, b: CellPos) -> Vec<CellPos> {
    debug_assert!(
        a.x == b.x || a.y == b.y,
        "straight_line needs an axis-aligned segment"
    );
    let dx = (b.x as i64 - a.x as i64).signum() as i32;
    let dy = (b.y as i64 - a.y as i64).signum() as i32;
    let steps = b.x.abs_diff(a.x).max(b.y.abs_diff(a.y));
    (0..=steps)
        .map(|i| {
            CellPos::new(
                a.x.wrapping_add_signed(dx * i as i32),
                a.y.wrapping_add_signed(dy * i as i32),
            )
        })
        .collect()
}

/// The arrowhead glyph pointing in the direction of travel into the target (ADR 013):
/// solid `< > ^ v` — svgbob content, so it never changes edge resolution.
fn arrowhead_glyph(toward: EdgeSet) -> char {
    match toward {
        EdgeSet::E => '>',
        EdgeSet::W => '<',
        EdgeSet::S => 'v',
        _ => '^', // N
    }
}

/// The edge direction from one route cell to its (adjacent) neighbor.
fn direction_toward(from: CellPos, to: CellPos) -> EdgeSet {
    if to.x > from.x {
        EdgeSet::E
    } else if to.x < from.x {
        EdgeSet::W
    } else if to.y > from.y {
        EdgeSet::S
    } else {
        EdgeSet::N
    }
}

/// Stamps one connector into the place-stage grid (called in document order from `place`).
pub(crate) fn place_connector(
    grid: &mut PlacedGrid,
    diagram: &Diagram,
    registry: &ShapeRegistry,
    connector: &ConnectorInstance,
) -> Result<(), RenderError> {
    // `style` (solid/dashed) is not honored in v1 — dashed rendering is an ADR 013 follow-up.
    let from = resolve_endpoint(diagram, registry, &connector.from)?;
    let to = resolve_endpoint(diagram, registry, &connector.to)?;
    let route = route(from.cell, to.cell, from.side, to.side);
    if route.len() < 2 {
        return Ok(()); // coincident endpoints: nothing to draw (ADR 013)
    }

    // Structural edges toward the route's neighbors — OR-merged with border/art cells
    // (F2), claiming the top so later objects can still cover this connector (F1).
    for (i, cell) in route.iter().enumerate() {
        let mut edges = EdgeSet::NONE;
        if i > 0 {
            edges |= direction_toward(*cell, route[i - 1]);
        }
        if i + 1 < route.len() {
            edges |= direction_toward(*cell, route[i + 1]);
        }
        grid.with_cell(cell.x, cell.y, |state| state.stamp_edges(edges));
    }

    // Arrowhead: content on the second-to-last cell, pointing at the target (ADR 013).
    if route.len() >= 3 {
        let base = route[route.len() - 2];
        let glyph = arrowhead_glyph(direction_toward(base, route[route.len() - 1]));
        grid.with_cell(base.x, base.y, |state| state.stamp_content(glyph));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "test code: panicking with the value on a failed construction is fine"
    )]

    use super::*;
    use crate::place::place;
    use diagram_model::geometry::{CellPos as MPos, CellSize as MSize};
    use diagram_model::objects::{ConnectorStyle, ObjectId, ShapeInstance};
    use shape_registry::{AnchorSet, Collection, GeometryKind, ShapeType, TemplateGrid};

    fn box_at(id: u32, x: u32, y: u32, w: u32, h: u32) -> ShapeInstance {
        ShapeInstance::new(
            ObjectId::generated(id),
            "box",
            "",
            MPos::new(x, y),
            MSize::new(w, h).expect("non-zero test size"),
        )
        .expect("valid shape")
    }

    fn ellipse_at(id: u32, x: u32, y: u32) -> ShapeInstance {
        ShapeInstance::new(
            ObjectId::generated(id),
            "ellipse",
            "",
            MPos::new(x, y),
            MSize::new(9, 5).expect("non-zero test size"),
        )
        .expect("valid shape")
    }

    fn endpoint(d: &Diagram, registry: &ShapeRegistry, object_id: u32, anchor: &str) -> Endpoint {
        let reference =
            AnchorRef::new(ObjectId::generated(object_id), anchor).expect("valid anchor");
        resolve_endpoint(d, registry, &reference).expect("resolvable endpoint")
    }

    #[test]
    fn rect_anchors_map_proportionally_at_any_size() {
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 2, 1, 5, 3)).unwrap(); // natural template is 9x5
        let registry = ShapeRegistry::builtin();
        assert_eq!(
            endpoint(&d, &registry, 0, "e"),
            Endpoint {
                cell: MPos::new(6, 2),
                side: Some(Side::Right)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 0, "n"),
            Endpoint {
                cell: MPos::new(4, 1),
                side: Some(Side::Top)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 0, "s"),
            Endpoint {
                cell: MPos::new(4, 3),
                side: Some(Side::Bottom)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 0, "w"),
            Endpoint {
                cell: MPos::new(2, 2),
                side: Some(Side::Left)
            }
        );
    }

    /// B-001: ellipse anchors map proportionally onto the instance extent, exactly like
    /// `Rect` (the template coordinates are the standard mid-side points).
    #[test]
    fn ellipse_anchors_map_proportionally_at_any_size() {
        let mut d = Diagram::new();
        let e = ShapeInstance::new(
            ObjectId::generated(0),
            "ellipse",
            "",
            CellPos::new(1, 2),
            CellSize::new(5, 3).expect("non-zero test size"),
        )
        .expect("valid shape");
        d.place_shape(e).unwrap();
        let registry = ShapeRegistry::builtin();
        assert_eq!(
            endpoint(&d, &registry, 0, "e"),
            Endpoint {
                cell: MPos::new(5, 3),
                side: Some(Side::Right)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 0, "n"),
            Endpoint {
                cell: MPos::new(3, 2),
                side: Some(Side::Top)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 0, "s"),
            Endpoint {
                cell: MPos::new(3, 4),
                side: Some(Side::Bottom)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 0, "w"),
            Endpoint {
                cell: MPos::new(1, 3),
                side: Some(Side::Left)
            }
        );
    }

    #[test]
    fn rect_anchors_map_to_themselves_at_natural_size() {
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 0, 0, 9, 5)).unwrap();
        let registry = ShapeRegistry::builtin();
        assert_eq!(endpoint(&d, &registry, 0, "e").cell, MPos::new(8, 2));
        assert_eq!(endpoint(&d, &registry, 0, "n").cell, MPos::new(4, 0));
    }

    #[test]
    fn pattern_anchors_are_plain_offsets() {
        let mut d = Diagram::new();
        d.place_shape(ellipse_at(1, 5, 7)).unwrap();
        let registry = ShapeRegistry::builtin();
        assert_eq!(
            endpoint(&d, &registry, 1, "s"),
            Endpoint {
                cell: MPos::new(9, 11),
                side: Some(Side::Bottom)
            }
        );
        assert_eq!(
            endpoint(&d, &registry, 1, "n"),
            Endpoint {
                cell: MPos::new(9, 7),
                side: Some(Side::Top)
            }
        );
        assert_eq!(endpoint(&d, &registry, 1, "w").cell, MPos::new(5, 9));
        assert_eq!(endpoint(&d, &registry, 1, "e").cell, MPos::new(13, 9));
    }

    #[test]
    fn unknown_anchor_name_is_a_typed_error() {
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 0, 0, 9, 5)).unwrap();
        let reference = AnchorRef::new(ObjectId::generated(0), "x").expect("valid anchor");
        let err = resolve_endpoint(&d, &ShapeRegistry::builtin(), &reference).unwrap_err();
        assert_eq!(
            err,
            RenderError::UnknownAnchor {
                object: ObjectId::generated(0),
                anchor: "x".to_owned()
            }
        );
    }

    #[test]
    fn anchor_outside_template_extent_is_a_typed_error() {
        let collection = Collection {
            name: "odd".to_owned(),
            types: vec![ShapeType {
                type_id: "odd/box".to_owned(),
                name: "box".to_owned(),
                geometry: GeometryKind::Rect,
                glyph_pattern: TemplateGrid::from_rows(&["----", "|  |", "----"])
                    .expect("valid art"),
                anchor_set: {
                    let mut a = AnchorSet::new();
                    a.add("out", shape_registry::CellPos::new(5, 0)).unwrap();
                    a
                },
                resize_rules: None,
            }],
        };
        let mut registry = ShapeRegistry::builtin();
        registry.add_collection(collection);

        let s = ShapeInstance::new(
            ObjectId::generated(0),
            "odd/box",
            "",
            MPos::new(0, 0),
            MSize::new(4, 3).expect("non-zero test size"),
        )
        .expect("valid shape");
        let mut d = Diagram::new();
        d.place_shape(s).unwrap();
        let reference = AnchorRef::new(ObjectId::generated(0), "out").expect("valid anchor");
        let err = resolve_endpoint(&d, &registry, &reference).unwrap_err();
        assert!(
            matches!(err, RenderError::AnchorOutsideExtent { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn interior_and_corner_anchors_have_no_side() {
        let collection = Collection {
            name: "anchored".to_owned(),
            types: vec![ShapeType {
                type_id: "anchored/sq".to_owned(),
                name: "sq".to_owned(),
                geometry: GeometryKind::Pattern,
                glyph_pattern: TemplateGrid::from_rows(&["----", "|  |", "----"])
                    .expect("valid art"),
                anchor_set: {
                    let mut a = AnchorSet::new();
                    a.add("center", shape_registry::CellPos::new(2, 1)).unwrap();
                    a.add("corner", shape_registry::CellPos::new(0, 0)).unwrap();
                    a
                },
                resize_rules: None,
            }],
        };
        let mut registry = ShapeRegistry::builtin();
        registry.add_collection(collection);

        let s = ShapeInstance::new(
            ObjectId::generated(0),
            "anchored/sq",
            "",
            MPos::new(0, 0),
            MSize::new(4, 3).expect("non-zero test size"),
        )
        .expect("valid shape");
        let mut d = Diagram::new();
        d.place_shape(s).unwrap();
        assert_eq!(endpoint(&d, &registry, 0, "center").side, None);
        assert_eq!(endpoint(&d, &registry, 0, "corner").side, None);
    }

    #[test]
    fn route_is_straight_when_aligned() {
        let cells = route(MPos::new(1, 2), MPos::new(7, 2), None, None);
        assert_eq!(cells.len(), 7);
        assert_eq!(cells.first().copied(), Some(MPos::new(1, 2)));
        assert_eq!(cells.last().copied(), Some(MPos::new(7, 2)));
        assert!(cells.iter().all(|c| c.y == 2));
    }

    #[test]
    fn route_corner_prefers_the_target_side() {
        // Target on a vertical side: the final segment is horizontal.
        let cells = route(MPos::new(0, 0), MPos::new(5, 3), None, Some(Side::Right));
        assert_eq!(cells[1], MPos::new(0, 1), "vertical first to (0,3)");
        assert_eq!(cells[3], MPos::new(0, 3), "corner at (from.x, to.y)");
        // Target on a horizontal side: the final segment is vertical.
        let cells = route(MPos::new(0, 0), MPos::new(5, 3), None, Some(Side::Bottom));
        assert_eq!(cells[1], MPos::new(1, 0), "horizontal first to (5,0)");
        assert_eq!(cells[5], MPos::new(5, 0), "corner at (to.x, from.y)");
    }

    #[test]
    fn route_corner_falls_back_to_the_source_side() {
        // Source departs horizontally: corner at (to.x, from.y).
        let cells = route(MPos::new(0, 0), MPos::new(5, 3), Some(Side::Right), None);
        assert_eq!(cells[1], MPos::new(1, 0));
        // Source departs vertically: corner at (from.x, to.y).
        let cells = route(MPos::new(0, 0), MPos::new(5, 3), Some(Side::Top), None);
        assert_eq!(cells[1], MPos::new(0, 1));
    }

    #[test]
    fn route_corner_defaults_to_vertical_first() {
        let cells = route(MPos::new(0, 0), MPos::new(5, 3), None, None);
        assert_eq!(cells[1], MPos::new(0, 1), "corner at (from.x, to.y)");
        assert_eq!(cells.len(), 9);
    }

    #[test]
    fn coincident_endpoints_produce_no_route() {
        assert!(route(MPos::new(2, 2), MPos::new(2, 2), None, None).is_empty());
    }

    #[test]
    fn arrowhead_glyphs_point_into_the_target() {
        assert_eq!(arrowhead_glyph(EdgeSet::E), '>');
        assert_eq!(arrowhead_glyph(EdgeSet::W), '<');
        assert_eq!(arrowhead_glyph(EdgeSet::S), 'v');
        assert_eq!(arrowhead_glyph(EdgeSet::N), '^');
    }

    #[test]
    fn connector_stamps_junctions_arrowhead_and_edges() {
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 0, 0, 9, 5)).unwrap();
        d.place_shape(box_at(1, 14, 0, 9, 5)).unwrap();
        d.connect(ConnectorInstance::new(
            ObjectId::generated(2),
            AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        ))
        .unwrap();
        let g = place(&d, &ShapeRegistry::builtin()).expect("known shapes");
        // A's e-anchor (right-side cell): border N|S plus the route's E -> T-junction.
        assert_eq!(g.cell(8, 2).edges, EdgeSet::N | EdgeSet::S | EdgeSet::E);
        // Plain route cell: W|E.
        assert_eq!(g.cell(10, 2).edges, EdgeSet::W | EdgeSet::E);
        // Arrowhead cell: edges W|E with content '>' stamped on top (not structural).
        assert_eq!(g.cell(13, 2).edges, EdgeSet::W | EdgeSet::E);
        assert_eq!(g.cell(13, 2).content, Some('>'));
        assert!(!g.cell(13, 2).structural_top, "the arrowhead is content");
        // B's w-anchor: border N|S plus the route's W.
        assert_eq!(g.cell(14, 2).edges, EdgeSet::N | EdgeSet::S | EdgeSet::W);
    }

    #[test]
    fn coincident_self_connector_draws_nothing() {
        // A 1x1 box: every standard anchor maps to the single cell, so e->n is coincident.
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 0, 0, 1, 1)).unwrap();
        d.connect(ConnectorInstance::new(
            ObjectId::generated(1),
            AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::generated(0), "n").expect("valid anchor"),
            ConnectorStyle::Solid,
        ))
        .unwrap();
        let with_link = place(&d, &ShapeRegistry::builtin()).expect("known shapes");
        let mut bare = Diagram::new();
        bare.place_shape(box_at(0, 0, 0, 1, 1)).unwrap();
        let without_link = place(&bare, &ShapeRegistry::builtin()).expect("known shapes");
        assert_eq!(with_link.width, without_link.width);
        assert_eq!(with_link.height, without_link.height);
        for y in 0..without_link.height {
            for x in 0..without_link.width {
                assert_eq!(with_link.cell(x, y), without_link.cell(x, y));
            }
        }
    }

    #[test]
    fn arrowhead_lands_on_the_second_to_last_cell() {
        // Boxes at x=0 and x=3 (1x1): route (0,0),(1,0),(2,0),(3,0) — 4 cells.
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 0, 0, 1, 1)).unwrap();
        d.place_shape(box_at(1, 3, 0, 1, 1)).unwrap();
        d.connect(ConnectorInstance::new(
            ObjectId::generated(2),
            AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        ))
        .unwrap();
        let g = place(&d, &ShapeRegistry::builtin()).expect("known shapes");
        assert_eq!(g.cell(1, 0).edges, EdgeSet::W | EdgeSet::E);
        assert_eq!(
            g.cell(2, 0).content,
            Some('>'),
            "arrowhead at second-to-last, pointing E"
        );
        assert!(!g.cell(2, 0).structural_top);
    }

    #[test]
    fn two_cell_route_carries_no_arrowhead() {
        let cells = route(MPos::new(0, 0), MPos::new(1, 0), None, None);
        assert_eq!(cells, vec![MPos::new(0, 0), MPos::new(1, 0)]);
        // Two adjacent 1x1 boxes: the route is exactly two cells, so no arrowhead.
        let mut d = Diagram::new();
        d.place_shape(box_at(0, 0, 0, 1, 1)).unwrap();
        d.place_shape(box_at(1, 1, 0, 1, 1)).unwrap();
        d.connect(ConnectorInstance::new(
            ObjectId::generated(2),
            AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        ))
        .unwrap();
        let g = place(&d, &ShapeRegistry::builtin()).expect("known shapes");
        assert_eq!(g.cell(0, 0).edges, EdgeSet::ALL);
        assert_eq!(g.cell(1, 0).edges, EdgeSet::ALL);
        assert_eq!(g.cell(0, 0).content, None, "no arrowhead on a 2-cell route");
        assert_eq!(g.cell(1, 0).content, None);
    }
}
