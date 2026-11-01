//! Property tests (S-006 AC1/AC4, S-007): the pipeline is pure and deterministic, and
//! every emitted character stays inside svgbob's ASCII input alphabet (ADR 001 v2 +
//! ADR 013 arrowheads). Random diagrams include anchor-bound connectors.
//!
//! Purity reading: svgbob consumes printable ASCII; characters outside its *drawing*
//! vocabulary degrade to literal text (ADR 001 consequences). So the invariant is:
//! every cell is a space or printable ASCII, and every non-alphanumeric character is a
//! structural/quote glyph of the svgbob alphabet. Labels are generated from a charset
//! that mixes plain letters/digits with drawing glyphs to exercise quote escaping.

use diagram_model::Diagram;
use diagram_model::anchors::{AnchorRef, ConnectorEndpoint};
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{
    ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance, TextInstance,
};
use proptest::prelude::*;
use shape_registry::ShapeRegistry;

/// Characters a generated label/text line may contain: plain text plus svgbob drawing
/// glyphs (to force quote escaping) and spaces.
const TEXT_CHARSET: &[u8] = b"abz0-9 +-|_()";

fn object_id(n: u32) -> ObjectId {
    ObjectId::generated(n)
}

fn shape(id: u32, label: String, x: u32, y: u32, w: u32, h: u32) -> Object {
    Object::Shape(
        ShapeInstance::new(
            object_id(id),
            "box",
            &label,
            CellPos::new(x, y),
            CellSize::new(w.max(1), h.max(1)).expect("non-zero size"),
        )
        .expect("valid shape"),
    )
}

fn text_obj(id: u32, content: String, x: u32, y: u32) -> Object {
    Object::Text(
        TextInstance::new(object_id(id), CellPos::new(x, y), &content).expect("valid text"),
    )
}

fn text_strategy() -> impl Strategy<Value = String> {
    // At least one character: the model rejects empty text objects.
    prop::collection::vec(proptest::sample::select(TEXT_CHARSET), 1..12)
        .prop_map(|v: Vec<u8>| String::from_utf8(v).expect("ascii charset"))
}

/// One random object slot: a box (possibly labeled) or a free-text object. `id` keeps
/// every object id unique within a diagram (boxes use 0..6, texts use 100+).
fn one_object(id: u32) -> impl Strategy<Value = Option<Object>> {
    (
        any::<bool>(),
        text_strategy(),
        0..24u32,
        0..16u32,
        0..12u32,
        0..8u32,
    )
        .prop_map(move |(is_box, label, x, y, w, h)| {
            Some(if is_box {
                shape(id, label, x, y, w, h)
            } else {
                text_obj(100 + id, label, x, y)
            })
        })
}

/// The four side anchors — every shape in the random diagrams is a box, so all exist.
const SIDE_ANCHORS: &[&str] = &["n", "e", "s", "w"];

/// A random diagram: up to 6 objects inside a 24x16 canvas, scale 1..3, plus up to two
/// solid connectors chaining consecutive boxes (ids 200+; anchors re-resolve at render).
fn random_diagram() -> impl Strategy<Value = Diagram> {
    (
        one_object(0),
        one_object(1),
        one_object(2),
        one_object(3),
        one_object(4),
        one_object(5),
        0..3u32,
    )
        .prop_map(|slots| {
            let connector_count = slots.6 as usize;
            let objects: Vec<Object> = [slots.0, slots.1, slots.2, slots.3, slots.4, slots.5]
                .into_iter()
                .flatten()
                .collect();
            let mut d = Diagram::new();
            // Deterministic per shape: scale 1..3.
            let scale = 1 + (objects.len() as u32 % 3);
            if scale != 1 {
                d.set_scale(scale).expect("scale >= 1");
            }
            let mut box_ids: Vec<ObjectId> = Vec::new();
            for obj in objects {
                match obj {
                    Object::Shape(s) => {
                        let id = s.id.clone();
                        d.place_shape(s).expect("unique ids");
                        box_ids.push(id);
                    }
                    Object::Text(t) => d.place_text(t).expect("unique ids"),
                    Object::Connector(_) => unreachable!(),
                }
            }
            // Chain up to `connector_count` consecutive boxes with random side anchors.
            for k in 0..connector_count.min(box_ids.len().saturating_sub(1)) {
                d.connect(ConnectorInstance::between(
                    object_id((200 + k) as u32),
                    AnchorRef::new(box_ids[k].clone(), SIDE_ANCHORS[k % 4]).expect("side anchor"),
                    AnchorRef::new(
                        box_ids[(k + 1) % box_ids.len()].clone(),
                        SIDE_ANCHORS[(k + 1) % 4],
                    )
                    .expect("side anchor"),
                    ConnectorStyle::Solid,
                ))
                .expect("endpoints are boxes");
            }
            // CR-010: one standalone arrow with both ends free, so the purity and
            // determinism properties also cover unattached endpoints.
            d.connect(ConnectorInstance::new(
                object_id(999),
                ConnectorEndpoint::Free(CellPos::new(3, 2)),
                ConnectorEndpoint::Free(CellPos::new(8, 5)),
                ConnectorStyle::Solid,
            ))
            .expect("free endpoints are always valid");
            d
        })
}

/// S-043 / ADR 018 — the operation vocabulary from `diagram-model`'s property tests,
/// so render properties also cover diagrams built through *operation sequences*.
fn op_strategy() -> impl Strategy<Value = diagram_model::testgen::Op> {
    use diagram_model::testgen::{ANCHORS, Op};
    prop_oneof![
        (0u32..32, 0u32..32, 0u32..12, 0u32..12).prop_map(|(x, y, w, h)| Op::PlaceShape {
            x,
            y,
            w,
            h
        }),
        (0u32..32, 0u32..32, any::<bool>()).prop_map(|(x, y, empty)| Op::PlaceText { x, y, empty }),
        (0usize..8, 0u32..40, 0u32..40).prop_map(|(shape_index, x, y)| Op::Move {
            shape_index,
            x,
            y
        }),
        (0usize..8, 0u32..16, 0u32..16).prop_map(|(shape_index, w, h)| Op::Resize {
            shape_index,
            w,
            h
        }),
        (0usize..12).prop_map(|object_index| Op::Remove { object_index }),
        (0usize..8, 0usize..8, any::<usize>(), any::<usize>()).prop_map(|(a, b, ai, bi)| {
            Op::Connect {
                a,
                b,
                anchor_a: ANCHORS[ai % ANCHORS.len()],
                anchor_b: ANCHORS[bi % ANCHORS.len()],
            }
        }),
        (0usize..6, 0u8..2, 0usize..8, any::<usize>()).prop_map(
            |(conn_index, end, shape_index, ai)| Op::AttachEnd {
                conn_index,
                end,
                shape_index,
                anchor: ANCHORS[ai % ANCHORS.len()]
            }
        ),
        (0usize..6, 0u8..2, 0u32..40, 0u32..40).prop_map(|(conn_index, end, x, y)| Op::FreeEnd {
            conn_index,
            end,
            x,
            y
        }),
        (0usize..6, 0usize..4, 0u32..40, 0u32..40).prop_map(|(conn_index, index, x, y)| {
            Op::AddBend {
                conn_index,
                index,
                x,
                y,
            }
        }),
        (0usize..6, 0usize..4, 0u32..40, 0u32..40).prop_map(|(conn_index, index, x, y)| {
            Op::MoveBend {
                conn_index,
                index,
                x,
                y,
            }
        }),
        (0usize..6, 0usize..4).prop_map(|(conn_index, index)| Op::RemoveBend { conn_index, index }),
        (0usize..6, 0u8..2, 0u8..4).prop_map(|(conn_index, end, style)| Op::SetTip {
            conn_index,
            end,
            style
        }),
        (0u32..10).prop_map(|scale| Op::SetScale { scale }),
        (
            any::<u8>(),
            any::<bool>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>()
        )
            .prop_map(|(role, set, r, g, b)| Op::SetColor { role, set, r, g, b }),
    ]
}

proptest! {
    /// AC1 — purity/determinism: rendering the same model twice yields byte-identical grids.
    #[test]
    fn render_is_deterministic(d in random_diagram()) {
        let registry = ShapeRegistry::builtin();
        let a = render_pipeline::render(&d, &registry).expect("renders");
        let b = render_pipeline::render(&d, &registry).expect("renders");
        prop_assert_eq!(a.to_text(), b.to_text());
        prop_assert_eq!(a.width(), b.width());
        prop_assert_eq!(a.height(), b.height());
    }

    /// AC4 — alphabet purity: every emitted character is a space or printable ASCII, and
    /// every non-alphanumeric character is a svgbob structural/quote glyph (ADR 001 v2).
    #[test]
    fn emitted_characters_stay_in_the_svgbob_alphabet(d in random_diagram()) {
        let grid = render_pipeline::render(&d, &ShapeRegistry::builtin()).expect("renders");
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                let c = grid.get(x, y).unwrap_or(' ');
                prop_assert!(
                    c == ' ' || c.is_ascii_graphic(),
                    "cell ({x},{y}) emits non-printable {c:?} — outside svgbob's input domain"
                );
                if !c.is_ascii_alphanumeric() && c != ' ' {
                    prop_assert!(
                        render_pipeline::is_svgbob_glyph(c),
                        "cell ({x},{y}) emits {c:?} — not a literal-text or svgbob glyph"
                    );
                }
            }
        }
    }

    /// Structural-only diagrams (no labels, no text) emit nothing but spaces and the
    /// structural line glyphs of the svgbob alphabet.
    #[test]
    fn structure_only_diagrams_emit_only_line_glyphs(
        boxes in prop::collection::vec((0..24u32, 0..16u32, 1..12u32, 1..8u32), 0..5)
    ) {
        let mut d = Diagram::new();
        for (i, (x, y, w, h)) in boxes.iter().enumerate() {
            d.place_shape(
                ShapeInstance::new(
                    object_id(i as u32),
                    "box",
                    "",
                    CellPos::new(*x, *y),
                    // Ranges above are 1-based, so the size is always non-zero.
                    CellSize::new(*w, *h).expect("non-zero size"),
                )
                .expect("valid shape"),
            )
            .expect("unique ids");
        }
        let grid = render_pipeline::render(&d, &ShapeRegistry::builtin()).expect("renders");
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                let c = grid.get(x, y).unwrap_or(' ');
                prop_assert!(
                    c == ' ' || render_pipeline::is_line_glyph(c),
                    "cell ({x},{y}) emits {c:?} — structural output must be line glyphs only"
                );
            }
        }
    }

    /// S-043 / ADR 018 — rendering an *operation-sequence* diagram (move/resize/connect/
    /// attach/detach/bends/tips/scale/colors via `diagram_model::testgen`) never panics:
    /// on success the grid is deterministic (two renders byte-equal) and alphabet-pure;
    /// on failure only typed `RenderError` variants occur — for these box-only diagrams
    /// that means a missing anchor (the vocabulary's `weird-anchor`) or an anchor outside
    /// a resized shape's extent.
    #[test]
    fn operation_generated_diagrams_render_deterministically_or_fail_typed(
        ops in prop::collection::vec(op_strategy(), 1..40)
    ) {
        let d = diagram_model::testgen::replay(&ops);
        match render_pipeline::render(&d, &ShapeRegistry::builtin()) {
            Ok(grid) => {
                let again =
                    render_pipeline::render(&d, &ShapeRegistry::builtin()).expect("renders twice");
                prop_assert_eq!(grid.to_text(), again.to_text());
                for y in 0..grid.height() {
                    for x in 0..grid.width() {
                        let c = grid.get(x, y).unwrap_or(' ');
                        // The operation vocabulary sets tip styles: the rhombi (ADR 013 v5)
                        // are intentional non-ASCII *content* glyphs, not svgbob alphabet —
                        // everything else must stay inside svgbob's printable domain.
                        prop_assert!(
                            c == ' ' || c.is_ascii_graphic() || c == '◇' || c == '◆',
                            "cell ({x},{y}) emits {c:?} — outside the allowed output alphabet"
                        );
                        if !c.is_ascii_alphanumeric() && c != ' ' && c != '◇' && c != '◆' {
                            prop_assert!(
                                render_pipeline::is_svgbob_glyph(c),
                                "cell ({x},{y}) emits {c:?} — not a literal-text or svgbob glyph"
                            );
                        }
                    }
                }
            }
            Err(err) => {
                let message = format!("{err:?}");
                prop_assert!(
                    matches!(
                        err,
                        render_pipeline::RenderError::UnknownAnchor { .. }
                            | render_pipeline::RenderError::AnchorOutsideExtent { .. }
                    ),
                    "unexpected render error for a box-only operation-sequence diagram: {message}"
                );
            }
        }
    }
}
