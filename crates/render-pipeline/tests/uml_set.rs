//! AC3 (S-036, CR-015): the built-in UML set stamps byte-for-byte at natural size
//! through the real pipeline, and connectors bind to their declared anchors. The
//! expected grid below is pinned verbatim — any art or anchor regression changes it.
//!
//! A connector endpoint overwrites its anchor cell with a junction (`+`), as in the
//! pre-existing resize tests; every other art cell must match the template exactly.

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance};
use render_pipeline::render;
use shape_registry::ShapeRegistry;

fn id(n: u32) -> ObjectId {
    ObjectId::generated(n)
}

/// A natural-size instance of built-in `type_id` at `(x, y)` (labels stay empty —
/// this test pins geometry and binding, not label layout).
fn shape(n: u32, type_id: &str, x: u32, y: u32) -> ShapeInstance {
    let registry = ShapeRegistry::builtin();
    let t = registry.lookup(type_id).expect("built-in");
    ShapeInstance::new(
        id(n),
        type_id,
        "",
        CellPos::new(x, y),
        CellSize::new(t.glyph_pattern.width(), t.glyph_pattern.height()).expect("non-zero"),
    )
    .expect("valid shape")
}

/// The full UML set, laid out in three rows with clear gaps:
///
/// ```text
/// row 1 (y=0..7):   lollipop(0,0) actor(5,0) activation(12,0) port(19,0) final(26,0) initial(30,3)
/// row 2 (y=12..19): decision(0,12) component(12,12) state(28,12)
/// row 3 (y=24..29): deployment(0,24) artifact(14,24)
/// ```
fn diagram() -> Diagram {
    let mut d = Diagram::new();
    for (n, (t, x, y)) in [
        ("lollipop", 0u32, 0u32),
        ("actor", 5, 0),
        ("activation", 12, 0),
        ("port", 19, 0),
        ("final", 26, 0),
        ("initial", 30, 3),
        ("decision", 0, 12),
        ("component", 12, 12),
        ("state", 28, 12),
        ("deployment", 0, 24),
        ("artifact", 14, 24),
    ]
    .into_iter()
    .enumerate()
    {
        d.place_shape(shape(n as u32 + 1, t, x, y)).expect("place");
    }
    // actor.w (grid cell 7,1) → lollipop.e (2,1): a straight horizontal segment.
    d.connect(ConnectorInstance::between(
        id(90),
        AnchorRef::new(id(2), "w").expect("anchor"),
        AnchorRef::new(id(1), "e").expect("anchor"),
        ConnectorStyle::Solid,
    ))
    .expect("endpoints exist");
    // decision.e (8,15) → component.w (12,13): an L — up from the diamond's east
    // tip, then along row 13 into the component's west edge.
    d.connect(ConnectorInstance::between(
        id(91),
        AnchorRef::new(id(7), "e").expect("anchor"),
        AnchorRef::new(id(8), "w").expect("anchor"),
        ConnectorStyle::Solid,
    ))
    .expect("endpoints exist");
    d
}

#[test]
fn uml_set_stamps_verbatim_and_connectors_bind_to_anchors() {
    let grid = render(&diagram(), &ShapeRegistry::builtin()).expect("renders");
    assert_eq!(grid.width(), 38);
    assert_eq!(grid.height(), 30);
    for (y, want) in [
        " _      o   +---+  *****   _          ",
        "( +<---+|\\  |   |  *****  (*)         ",
        "`-'    / \\  |   |  *****  `-'         ",
        " |     / \\  |   |             *       ",
        " |          |   |                     ",
        " |          |   |                     ",
        " |          +---+                     ",
        "                                      ",
        "                                      ",
        "                                      ",
        "                                      ",
        "                                      ",
        "   / \\      +--+--------+    .-------.",
        "  /   \\ +-->+  |        |    |       |",
        " /     \\|   +--+        |    |       |",
        "/       |      |        |    |       |",
        "\\       /      +--------+    `-------'",
        " \\     /                              ",
        "  \\   /                               ",
        "   \\ /                                ",
        "                                      ",
        "                                      ",
        "                                      ",
        "                                      ",
        "  _________   +-----                  ",
        " /       /    |     /                 ",
        "+-------+     |       +               ",
        "|       |     |       |               ",
        "+-------+     |       |               ",
        "              +-------+               ",
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(grid.row(y as u32), Some(want.to_owned()), "row {y}");
    }
}
