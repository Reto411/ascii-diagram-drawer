//! Golden file tests (S-006 AC2/AC3/AC5): every scenario below renders to the exact bytes
//! stored in `tests/golden/<name>.grid` — a row-major dump of the resolved grid, one line
//! per row, LF newlines, no trailing newline.
//!
//! Regenerate deliberately with `UPDATE_GOLDEN=1 cargo test -p render-pipeline --test golden`
//! and review the diff before committing.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: golden files are trusted fixtures"
)]

use std::fs;
use std::path::Path;

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{
    ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance, TextInstance,
};
use shape_registry::ShapeRegistry;

const GOLDEN_DIR: &str = "tests/golden";

/// The natural size of the built-in ellipse's pattern art (9x5).
const ELLIPSE_SIZE: (u32, u32) = (9, 5);

fn box_obj(id: u32, label: &str, x: u32, y: u32, w: u32, h: u32) -> Object {
    Object::Shape(
        ShapeInstance::new(
            ObjectId::generated(id),
            "box",
            label,
            CellPos::new(x, y),
            CellSize::new(w, h).expect("non-zero test size"),
        )
        .expect("valid shape"),
    )
}

fn ellipse(id: u32, label: &str, x: u32, y: u32) -> Object {
    Object::Shape(
        ShapeInstance::new(
            ObjectId::generated(id),
            "ellipse",
            label,
            CellPos::new(x, y),
            CellSize::new(ELLIPSE_SIZE.0, ELLIPSE_SIZE.1).expect("non-zero test size"),
        )
        .expect("valid shape"),
    )
}

fn text(id: u32, x: u32, y: u32, content: &str) -> Object {
    Object::Text(
        TextInstance::new(ObjectId::generated(id), CellPos::new(x, y), content)
            .expect("valid text"),
    )
}

fn diagram(scale: u32, objects: Vec<Object>) -> Diagram {
    let mut d = Diagram::new();
    if scale != 1 {
        d.set_scale(scale).expect("scale >= 1");
    }
    for o in objects {
        match o {
            Object::Shape(s) => d.place_shape(s).expect("place shape"),
            Object::Text(t) => d.place_text(t).expect("place text"),
            Object::Connector(c) => d.connect(c).expect("connect connector"),
        }
    }
    d
}

/// Adds one solid connector to a diagram built by `diagram` (S-007, ADR 013): endpoint
/// ids are the generated ids of the scenario's shapes; the connector itself is id 90.
fn linked(mut d: Diagram, from_id: u32, from_anchor: &str, to_id: u32, to_anchor: &str) -> Diagram {
    d.connect(ConnectorInstance::new(
        ObjectId::generated(90),
        AnchorRef::new(ObjectId::generated(from_id), from_anchor).expect("valid anchor"),
        AnchorRef::new(ObjectId::generated(to_id), to_anchor).expect("valid anchor"),
        ConnectorStyle::Solid,
    ))
    .expect("endpoints are placed shapes");
    d
}

/// One golden scenario: a name plus the exact diagram it freezes.
fn scenarios() -> Vec<(&'static str, Diagram)> {
    vec![
        // A single box: corners and straights only (C1 rule table basics).
        ("box", diagram(1, vec![box_obj(0, "Hello", 0, 0, 9, 5)])),
        // Pattern art stamped verbatim with a label on its space cells (ADR 007).
        ("ellipse", diagram(1, vec![ellipse(0, "DB", 0, 0)])),
        // AC2: the mixed diagram — box + ellipse + free text (quoted) in one render.
        (
            "mixed-diagram",
            diagram(
                1,
                vec![
                    box_obj(0, "User", 0, 0, 9, 5),
                    ellipse(1, "DB", 14, 1),
                    text(2, 0, 8, "note: a+b"),
                ],
            ),
        ),
        // Cross junction: the narrow box's column pierces the big box's border; the
        // merged cell (all four directions) resolves to '+' (priority: cross first).
        (
            "cross-junction",
            diagram(
                1,
                vec![box_obj(0, "", 0, 0, 9, 5), box_obj(1, "", 4, 0, 1, 7)],
            ),
        ),
        // T-junction: the narrow box hangs off the middle of the big box's bottom edge.
        (
            "t-junction",
            diagram(
                1,
                vec![box_obj(0, "", 0, 0, 9, 3), box_obj(1, "", 4, 2, 1, 3)],
            ),
        ),
        // F1 (content over edges): text stamped after the border overpaints it.
        (
            "content-over-edge",
            diagram(1, vec![box_obj(0, "", 0, 0, 9, 5), text(1, 3, 4, "hi")]),
        ),
        // F1 reversed (edges over content): a border stamped after the text covers it.
        (
            "edge-over-content",
            diagram(1, vec![text(0, 3, 4, "hi"), box_obj(1, "", 0, 0, 9, 5)]),
        ),
        // F3: a label wider than the interior clips deterministically (floor centering).
        (
            "label-clipped",
            diagram(1, vec![box_obj(0, "abcdefgh", 0, 0, 7, 3)]),
        ),
        // F4 (quote fits): an svgbob-alphabet label is quoted and still fits.
        (
            "label-quote-fits",
            diagram(1, vec![box_obj(0, "a+b", 0, 0, 9, 3)]),
        ),
        // F4 (quote fallback): the quoted form does not fit, so the raw line is stamped.
        (
            "label-quote-fallback",
            diagram(1, vec![box_obj(0, "-x-", 0, 0, 5, 3)]),
        ),
        // AC5: scale 2 with a box + label + junction — lines stay continuous (each line
        // glyph fills its whole 2x2 block) and the label lands in its defined sub-cells.
        (
            "scale-2",
            diagram(
                2,
                vec![box_obj(0, "AB", 0, 0, 9, 5), box_obj(1, "", 6, 0, 1, 7)],
            ),
        ),
        // S-007 (ADR 013): straight horizontal route, T-junctions at both anchors,
        // arrowhead '>' on the second-to-last cell.
        (
            "connector-horizontal",
            linked(
                diagram(
                    1,
                    vec![box_obj(0, "A", 0, 0, 9, 5), box_obj(1, "B", 14, 0, 9, 5)],
                ),
                0,
                "e",
                1,
                "w",
            ),
        ),
        // S-007: straight vertical route between two ellipse anchors (pattern base edges
        // merge with the route into T-junctions), arrowhead 'v'.
        (
            "connector-vertical",
            linked(
                diagram(1, vec![ellipse(0, "DB", 0, 0), ellipse(1, "API", 0, 12)]),
                0,
                "s",
                1,
                "n",
            ),
        ),
        // S-007: misaligned endpoints — single-corner L; the target's left side fixes the
        // corner (vertical first, then horizontal into B.w).
        (
            "connector-diagonal",
            linked(
                diagram(
                    1,
                    vec![box_obj(0, "A", 0, 0, 9, 5), box_obj(1, "B", 14, 6, 9, 5)],
                ),
                0,
                "e",
                1,
                "w",
            ),
        ),
        // S-007: anchors on the same column — straight vertical route down A.s into B.n.
        (
            "connector-mid-edge",
            linked(
                diagram(
                    1,
                    vec![box_obj(0, "A", 0, 0, 9, 5), box_obj(1, "B", 0, 10, 9, 5)],
                ),
                0,
                "s",
                1,
                "n",
            ),
        ),
        // S-007 at scale 2: the route expands like any line; the arrowhead is content and
        // lands in its defined sub-cell (the block keeps no line glyph around it).
        (
            "connector-scale-2",
            linked(
                diagram(
                    2,
                    vec![box_obj(0, "A", 0, 0, 9, 5), box_obj(1, "B", 14, 0, 9, 5)],
                ),
                0,
                "e",
                1,
                "w",
            ),
        ),
    ]
}

#[test]
fn goldens_match_the_rendered_bytes() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    for (name, d) in scenarios() {
        let path = Path::new(GOLDEN_DIR).join(format!("{name}.grid"));
        let grid = render_pipeline::render(&d, &ShapeRegistry::builtin())
            .unwrap_or_else(|err| panic!("{name}: {err}"));
        let expected = format!("{}\n", grid.to_text());

        if update {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("creates golden dir");
            }
            fs::write(&path, &expected).expect("writes golden file");
            continue;
        }
        let actual = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "missing golden file {} — run with UPDATE_GOLDEN=1",
                path.display()
            )
        });
        assert_eq!(
            actual, expected,
            "{name}: rendered grid differs from the golden file"
        );
    }
}
