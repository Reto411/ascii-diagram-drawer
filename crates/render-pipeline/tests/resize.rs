//! AC3 (S-024, CR-003): rule-bearing custom templates are re-rasterized at the
//! instance's own pos/size in the place stage — out-of-range sizes clamp to the
//! minimum extent instead of failing, anchors map proportionally (like the built-in
//! Ellipse, B-001), and labels land over the rasterized extent. Rule-free templates
//! keep their natural-size stamp byte-for-byte.

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance};
use render_pipeline::render;
use shape_registry::CellPos as RegCellPos;
use shape_registry::{
    AnchorSet, Collection, EdgeRule, GeometryKind, ResizeRules, ShapeRegistry, ShapeType,
    TemplateGrid,
};

/// The canonical box: `+--+` / `|  |` / `+--+` with the matching rule set.
fn box_rules() -> ResizeRules {
    let edge = |c: char| EdgeRule {
        insert: c,
        remove: c,
        even: None,
        odd: None,
    };
    ResizeRules {
        top: edge('-'),
        bottom: edge('-'),
        left: edge('|'),
        right: edge('|'),
    }
}

fn box_grid() -> TemplateGrid {
    TemplateGrid::from_rows(&["+--+", "|  |", "+--+"]).expect("valid")
}

/// A registry whose `c/box` is the rule-bearing box; `c/plain` is a rule-free pattern.
fn registry() -> ShapeRegistry {
    let mut anchors = AnchorSet::new();
    anchors.add("n", RegCellPos::new(2, 0)).expect("fresh");
    anchors.add("e", RegCellPos::new(3, 1)).expect("fresh");

    let box_type = ShapeType {
        type_id: "c/box".to_owned(),
        name: "box".to_owned(),
        glyph_pattern: box_grid(),
        anchor_set: anchors,
        geometry: GeometryKind::Pattern,
        resize_rules: Some(box_rules()),
    };
    let plain = ShapeType {
        type_id: "c/plain".to_owned(),
        name: "plain".to_owned(),
        glyph_pattern: TemplateGrid::from_rows(&["ab", "cd"]).expect("valid"),
        anchor_set: AnchorSet::new(),
        geometry: GeometryKind::Pattern,
        resize_rules: None,
    };
    let mut registry = ShapeRegistry::builtin();
    registry.add_collection(Collection {
        name: "c".to_owned(),
        types: vec![box_type, plain],
    });
    registry
}

fn shape(id: u32, type_id: &str, label: &str, x: u32, y: u32, w: u32, h: u32) -> ShapeInstance {
    ShapeInstance::new(
        ObjectId::generated(id),
        type_id,
        label,
        CellPos::new(x, y),
        CellSize::new(w.max(1), h.max(1)).expect("non-zero"),
    )
    .expect("valid shape")
}

/// AC3: a rule-bearing instance at a non-natural size renders the re-rasterized art —
/// the box grows to 6×5 with stretched runs.
#[test]
fn resized_rule_bearing_shape_re_rasterizes() {
    let mut d = Diagram::new();
    d.place_shape(shape(1, "c/box", "", 0, 0, 6, 5))
        .expect("place");
    let grid = render(&d, &registry()).expect("renders");
    assert_eq!(grid.width(), 6);
    assert_eq!(grid.height(), 5);
    for (y, want) in ["+----+", "|    |", "|    |", "|    |", "+----+"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(grid.row(y as u32), Some(want.to_owned()));
    }
}

/// AC3: out-of-range sizes clamp to the minimum extent — a 1×1 instance of the box
/// (minimum 2×2) renders at the minimum instead of failing or vanishing.
#[test]
fn out_of_range_sizes_clamp_to_the_minimum() {
    let mut d = Diagram::new();
    d.place_shape(shape(1, "c/box", "", 0, 0, 1, 1))
        .expect("place");
    let grid = render(&d, &registry()).expect("renders");
    assert_eq!(grid.width(), 2);
    assert_eq!(grid.height(), 2);
    // Width shrink eats the `-` runs; height shrink removes the single interior row —
    // what remains is top and bottom edge, both `++`.
    assert_eq!(grid.row(0), Some("++".to_owned()));
    assert_eq!(grid.row(1), Some("++".to_owned()));
}

/// AC3: anchors map proportionally onto the resized extent — a connector bound to the
/// box's `e` anchor (natural 3,1) endpoints at the proportional cell of the 8×6 resize.
#[test]
fn anchors_map_proportionally_on_resized_shapes() {
    let mut d = Diagram::new();
    d.place_shape(shape(1, "c/box", "", 0, 0, 8, 6))
        .expect("place");
    d.connect(ConnectorInstance::new(
        ObjectId::generated(90),
        AnchorRef::new(ObjectId::generated(1), "e").expect("anchor"),
        AnchorRef::new(ObjectId::generated(1), "n").expect("anchor"),
        ConnectorStyle::Solid,
    ))
    .expect("endpoints exist");
    let grid = render(&d, &registry()).expect("renders");
    // `e` at natural (3,1) of 4×3 → x: round(3·7/3)=7 (right edge), y: round(1·5/2)=3.
    // `n` at natural (2,0) → x: round(2·7/3)=5, y: 0. Target side = top, so the L
    // corner is (5,3): horizontal from (7,3) west to (5,3), then vertical north into
    // (5,0) with the arrowhead on the second-to-last cell (5,1).
    assert_eq!(grid.row(0), Some("+----+-+".to_owned()));
    assert_eq!(grid.row(1), Some("|    ^ |".to_owned()));
    assert_eq!(grid.row(2), Some("|    | |".to_owned()));
    assert_eq!(grid.row(3), Some("|    +-+".to_owned()));
    assert_eq!(grid.row(4), Some("|      |".to_owned()));
    assert_eq!(grid.row(5), Some("+------+".to_owned()));
}

/// AC3: the label centers over the rasterized extent — `hi` lands in the middle of the
/// 6×5 box's interior space cells.
#[test]
fn labels_land_over_the_rasterized_extent() {
    let mut d = Diagram::new();
    d.place_shape(shape(1, "c/box", "hi", 0, 0, 6, 5))
        .expect("place");
    let grid = render(&d, &registry()).expect("renders");
    // 5 rows, one label line → vertical offset (5-1)/2 = 2; row 2 is `|    |`.
    assert_eq!(grid.row(2), Some("| hi |".to_owned()));
}

/// AC3: rule-free templates keep today's behavior — the instance size is ignored and
/// the pattern stamps at its natural size, byte-for-byte.
#[test]
fn rule_free_templates_stamp_at_natural_size() {
    let mut d = Diagram::new();
    // A deliberately non-natural instance size: still stamped 2×2.
    d.place_shape(shape(1, "c/plain", "", 0, 0, 9, 9))
        .expect("place");
    let grid = render(&d, &registry()).expect("renders");
    assert_eq!(grid.width(), 2);
    assert_eq!(grid.height(), 2);
    assert_eq!(grid.row(0), Some("ab".to_owned()));
    assert_eq!(grid.row(1), Some("cd".to_owned()));
}
