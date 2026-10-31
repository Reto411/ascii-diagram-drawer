//! The resolve stage: one glyph per cell (C1, ADR 001).
//!
//! The rule table maps each of the 16 N/E/S/W connection states to exactly one
//! svgbob-alphabet glyph. Content (shape art, labels, text) wins over edges unless the
//! last writer was structural — that is z-order for ambiguous cells (C1 F1/F2).

use crate::edges::EdgeSet;
use crate::grid::ResolvedGrid;
use crate::place::{CellState, PlacedGrid};

/// Resolves an edge set to its glyph — the C1 rule table (all 16 connection states).
///
/// Priority: cross > T-junction > corner > straight. All junctions are sharp `+` in v1
/// (rounded joins exist only as template art — ADR 001); a single dangling direction is
/// its straight glyph (connector endpoints, S-007).
#[must_use]
pub(crate) fn resolve_edges(edges: EdgeSet) -> char {
    if edges.is_empty() {
        return ' ';
    }
    let n = edges.contains(EdgeSet::N);
    let e = edges.contains(EdgeSet::E);
    let s = edges.contains(EdgeSet::S);
    let w = edges.contains(EdgeSet::W);
    match (n, e, s, w) {
        // Straight: exactly two opposite directions.
        (true, false, true, false) => '|',
        (false, true, false, true) => '-',
        // Two adjacent (corner), three (T-junction) or four (cross): sharp '+'.
        _ if edges.count() >= 2 => '+',
        // One dangling direction: its straight glyph.
        _ => {
            if n || s {
                '|'
            } else {
                '-'
            }
        }
    }
}

/// Resolves one placed cell to its final glyph (C1): content wins unless the last writer
/// was structural (z-order, F1/F2); otherwise the edge set resolves via the rule table.
fn resolve_cell(cell: &CellState) -> char {
    if !cell.structural_top {
        return cell.content.unwrap_or(' ');
    }
    resolve_edges(cell.edges)
}

/// Resolves a placed grid to its final character grid (C2): trims to the bounding box of
/// occupied cells (the rendered extent), then resolves each cell row-major.
pub(crate) fn resolve(placed: &PlacedGrid) -> ResolvedGrid {
    if placed.cells.is_empty() {
        return ResolvedGrid::empty();
    }
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = u32::MIN;
    let mut max_y = u32::MIN;
    for y in 0..placed.height {
        for x in 0..placed.width {
            if placed.cell(x, y).occupied() {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x == u32::MAX {
        return ResolvedGrid::empty();
    }
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let mut cells = Vec::with_capacity((width as usize) * (height as usize));
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            cells.push(resolve_cell(placed.cell(x, y)));
        }
    }
    ResolvedGrid::new(width, height, cells)
}

#[cfg(test)]
mod tests {
    use super::{resolve, resolve_edges};
    use crate::edges::EdgeSet;
    use crate::grid::ResolvedGrid;
    use crate::place::place;
    use diagram_model::Diagram;
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::{Object, ObjectId, ShapeInstance, TextInstance};
    use shape_registry::ShapeRegistry;

    /// Every row of the C1 rule table: all 16 connection states → exactly one glyph.
    #[test]
    fn rule_table_covers_all_sixteen_states() {
        // Straight.
        assert_eq!(resolve_edges(EdgeSet::N | EdgeSet::S), '|');
        assert_eq!(resolve_edges(EdgeSet::E | EdgeSet::W), '-');
        // Corners (two adjacent directions) → sharp '+'.
        for edges in [
            EdgeSet::N | EdgeSet::E,
            EdgeSet::N | EdgeSet::W,
            EdgeSet::S | EdgeSet::E,
            EdgeSet::S | EdgeSet::W,
        ] {
            assert_eq!(resolve_edges(edges), '+', "corner {edges:?}");
        }
        // T-junctions (three directions) → sharp '+'.
        for edges in [
            EdgeSet::N | EdgeSet::E | EdgeSet::S,
            EdgeSet::N | EdgeSet::W | EdgeSet::S,
            EdgeSet::N | EdgeSet::E | EdgeSet::W,
            EdgeSet::S | EdgeSet::E | EdgeSet::W,
        ] {
            assert_eq!(resolve_edges(edges), '+', "T-junction {edges:?}");
        }
        // Cross (four directions) → sharp '+'.
        assert_eq!(resolve_edges(EdgeSet::ALL), '+');
        // Dangling stubs (single direction) → their straight glyph.
        assert_eq!(resolve_edges(EdgeSet::N), '|');
        assert_eq!(resolve_edges(EdgeSet::S), '|');
        assert_eq!(resolve_edges(EdgeSet::E), '-');
        assert_eq!(resolve_edges(EdgeSet::W), '-');
        // Empty → blank.
        assert_eq!(resolve_edges(EdgeSet::NONE), ' ');
    }

    /// A box object with an explicit id (diagram ids must be unique per diagram).
    fn box_obj(id: u32, x: u32, y: u32, w: u32, h: u32) -> Object {
        Object::Shape(
            ShapeInstance::new(
                ObjectId::generated(id),
                "box",
                "",
                CellPos::new(x, y),
                CellSize::new(w, h).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
    }

    fn render_one(objects: Vec<Object>) -> ResolvedGrid {
        let mut d = Diagram::new();
        for o in objects {
            match o {
                Object::Shape(s) => d.place_shape(s).unwrap(),
                Object::Text(t) => d.place_text(t).unwrap(),
                Object::Connector(c) => d.connect(c).unwrap(),
            }
        }
        resolve(&place(&d, &ShapeRegistry::builtin()).expect("known shapes"))
    }

    #[test]
    fn box_resolves_to_its_border_glyphs() {
        let g = render_one(vec![box_obj(0, 0, 0, 5, 3)]);
        assert_eq!(g.to_text(), "+---+\n|   |\n+---+");
    }

    #[test]
    fn resolve_trims_to_occupied_extent() {
        let g = render_one(vec![box_obj(0, 7, 3, 3, 2)]);
        assert_eq!(g.width(), 3);
        assert_eq!(g.height(), 2);
        assert_eq!(g.to_text(), "+-+\n+-+");
    }

    #[test]
    fn resolve_empty_diagram_is_empty_grid() {
        let g = resolve(&place(&Diagram::new(), &ShapeRegistry::builtin()).unwrap());
        assert!(g.is_empty());
    }

    #[test]
    fn content_wins_over_edges_unless_structural_top() {
        // Text over a border: the character wins (F1).
        let t =
            TextInstance::new(ObjectId::generated(9), CellPos::new(2, 0), "X").expect("valid text");
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
        d.place_text(t).unwrap();
        let g = resolve(&place(&d, &ShapeRegistry::builtin()).unwrap());
        // Top border row: +-X-+  (X replaces the `-` under it)
        assert_eq!(g.row(0), Some("+-X-+".to_owned()));

        // Border over text: the edge glyph wins (F2).
        let mut d = Diagram::new();
        d.place_text(
            TextInstance::new(ObjectId::generated(9), CellPos::new(2, 0), "X").expect("valid text"),
        )
        .unwrap();
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
        let g = resolve(&place(&d, &ShapeRegistry::builtin()).unwrap());
        assert_eq!(g.row(0), Some("+---+".to_owned()));
    }

    #[test]
    fn ellipse_resolves_to_its_verbatim_art() {
        let e = Object::Shape(
            ShapeInstance::new(
                ObjectId::generated(0),
                "ellipse",
                "",
                CellPos::new(0, 0),
                CellSize::new(9, 5).expect("non-zero test size"),
            )
            .expect("valid shape"),
        );
        let g = render_one(vec![e]);
        assert_eq!(g.width(), 9); // the ellipse's natural art size
        assert_eq!(g.height(), 5);
        assert_eq!(
            g.to_text(),
            "   ___   \n ,'   '. \n(       )\n `.   .' \n   `-'   "
        );
    }

    /// B-001: an ellipse instance at a size other than its natural 9×5 renders the
    /// outline re-rasterized at the instance extent (previously it was stamped at
    /// natural size, so resizing was a no-op).
    #[test]
    fn resized_ellipse_renders_at_instance_extent() {
        let e = Object::Shape(
            ShapeInstance::new(
                ObjectId::generated(0),
                "ellipse",
                "",
                CellPos::new(0, 0),
                CellSize::new(13, 8).expect("non-zero test size"),
            )
            .expect("valid shape"),
        );
        let g = render_one(vec![e]);
        assert_eq!(g.width(), 13);
        assert_eq!(g.height(), 8);
        assert_eq!(
            g.to_text(),
            "    ____     \n ,'       '. \n(           )\n(           )\n(           )\n(           )\n `.       .' \n    `--'     "
        );
    }

    #[test]
    fn cross_forms_where_borders_cross() {
        // A 1-wide box (a vertical line) crossing the big box's bottom edge: at the
        // crossing cell the edges merge to all four directions → '+'.
        let g = render_one(vec![box_obj(0, 0, 0, 9, 5), box_obj(1, 4, 0, 1, 7)]);
        assert_eq!(g.get(4, 4), Some('+'));
        // The crossing cell of the small box's top edge with the big box's bottom edge
        // is the same junction; elsewhere each line keeps its own glyph.
        assert_eq!(
            g.get(2, 4),
            Some('-'),
            "big box bottom edge stays a straight"
        );
        assert_eq!(g.get(4, 1), Some('|'), "1-wide box column stays a vertical");
    }

    #[test]
    fn corner_cells_resolve_to_sharp_plus() {
        // A small box nested inside a big one: its corners are '+' (sharp in v1).
        let g = render_one(vec![box_obj(0, 0, 0, 9, 7), box_obj(1, 4, 2, 3, 3)]);
        assert_eq!(g.get(4, 2), Some('+'));
        assert_eq!(g.get(6, 4), Some('+'));
    }

    #[test]
    fn adjacent_boxes_share_a_wall() {
        let g = render_one(vec![box_obj(0, 0, 0, 5, 4), box_obj(1, 5, 0, 5, 4)]);
        // The shared wall: column x=4 (N|S) and x=5 (N|S) — two parallel lines.
        assert_eq!(g.get(4, 2), Some('|'));
        assert_eq!(g.get(5, 2), Some('|'));
    }
}
