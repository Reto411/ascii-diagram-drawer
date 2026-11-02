//! Lossless round-trip property (S-004 AC3): for any well-formed [`Diagram`],
//! `serialize(parse(serialize(d))) == serialize(d)` and the parsed model equals `d`.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "test code: generated values are valid by construction"
)]

use diagram_model::{
    Diagram,
    anchors::{AnchorRef, ConnectorEndpoint},
    geometry::{CellPos, CellSize},
    objects::{ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance, TextInstance},
};
use proptest::prelude::*;

/// A unique object id: `<prefix><n>`, valid by construction (single token, no collisions).
fn id(prefix: &str, n: u32) -> ObjectId {
    ObjectId::new(&format!("{prefix}{n}")).expect("generated id is a single token")
}

type ShapeSpec = (String, String, u32, u32, u32, u32);
type TextSpec = (u32, u32, String);
/// `(from-shape-index, to-shape-index, from-anchor, to-anchor, dashed, free-from, free-to)` —
/// the free flags select CR-010 standalone ends instead of anchor refs.
type ConnectorSpec = (usize, usize, String, String, bool, bool, bool);

fn shape_spec() -> impl Strategy<Value = ShapeSpec> {
    (
        prop::sample::select(&["box", "circle", "diamond"]).prop_map(str::to_owned),
        "[a-z]{1,8}",
        0u32..20,
        0u32..20,
        1u32..10,
        1u32..10,
    )
}

/// Text content: plain words plus the tricky escape cases (quotes, backslashes, newlines).
fn text_content() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z]{1,8}",
        Just("a \"quoted\" word and a \\ backslash".to_owned()),
        Just("first line\nsecond line".to_owned()),
    ]
}

fn diagram_strategy() -> impl Strategy<Value = Diagram> {
    (
        1u32..8,
        prop::collection::vec(shape_spec(), 0..4),
        prop::collection::vec((0u32..20, 0u32..20, text_content()), 0..3),
        prop::collection::vec(
            (
                0usize..4,
                0usize..4,
                "[a-z-]{1,6}",
                "[a-z-]{1,6}",
                any::<bool>(),
                any::<bool>(),
                any::<bool>(),
            ),
            0..3,
        ),
    )
        .prop_map(|(scale, shapes, texts, conns)| build(scale, shapes, texts, conns))
}

/// Builds a well-formed diagram from generated specs; connectors only reference existing shapes.
fn build(
    scale: u32,
    shapes: Vec<ShapeSpec>,
    texts: Vec<TextSpec>,
    conns: Vec<ConnectorSpec>,
) -> Diagram {
    let mut diagram = Diagram::new();
    diagram.set_scale(scale).expect("scale >= 1");
    for (i, (type_id, label, x, y, w, h)) in shapes.iter().enumerate() {
        let size = CellSize::new(*w, *h).expect("generated size is non-zero");
        diagram
            .place_shape(
                ShapeInstance::new(
                    id("s", u32::try_from(i).expect("small index") + 1),
                    type_id,
                    label,
                    CellPos::new(*x, *y),
                    size,
                )
                .expect("generated shape is valid"),
            )
            .expect("valid shape");
    }
    for (i, (x, y, content)) in texts.iter().enumerate() {
        diagram
            .place_text(
                TextInstance::new(
                    id("t", u32::try_from(i).expect("small index") + 1),
                    CellPos::new(*x, *y),
                    content,
                )
                .expect("generated text is valid"),
            )
            .expect("valid text");
    }
    for (i, (a, b, from_anchor, to_anchor, dashed, free_from, free_to)) in conns.iter().enumerate()
    {
        if *a >= shapes.len() || *b >= shapes.len() {
            continue; // generated index out of range: skip this connector
        }
        let from = if *free_from {
            ConnectorEndpoint::Free(CellPos::new(10, 5))
        } else {
            ConnectorEndpoint::Attached(
                AnchorRef::new(
                    id("s", u32::try_from(*a).expect("small index") + 1),
                    from_anchor,
                )
                .expect("valid anchor"),
            )
        };
        let to = if *free_to {
            // Offset from the free source cell — both ends free at one cell is a
            // zero-length connector (CR-010) and would be rejected by `connect`.
            ConnectorEndpoint::Free(CellPos::new(11, 5))
        } else {
            ConnectorEndpoint::Attached(
                AnchorRef::new(
                    id("s", u32::try_from(*b).expect("small index") + 1),
                    to_anchor,
                )
                .expect("valid anchor"),
            )
        };
        diagram
            .connect(ConnectorInstance::new(
                id("c", u32::try_from(i).expect("small index") + 1),
                from,
                to,
                if *dashed {
                    ConnectorStyle::Dashed
                } else {
                    ConnectorStyle::Solid
                },
            ))
            .expect("valid connector");
    }
    diagram
}

/// S-043 / ADR 018 — the same operation vocabulary as `diagram-model`'s property tests,
/// so losslessness is checked on diagrams built through *operation sequences* (move,
/// resize, attach/detach, bends, tips, scale, colors) rather than direct specs.
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
    #[test]
    fn round_trip_is_lossless(d in diagram_strategy()) {
        let text = ad_format::serialize(&d);
        let parsed = ad_format::parse(&text)
            .unwrap_or_else(|err| panic!("canonical serialization must parse: {err}"));
        // Byte-stable re-save (QS7).
        prop_assert_eq!(ad_format::serialize(&parsed), text);
        // Model equality covers objects, order, geometry, labels and anchors.
        prop_assert_eq!(parsed, d);
    }

    /// S-043 / ADR 018 — losslessness also holds for diagrams built through random
    /// *operation sequences* (not just directly generated specs): after any sequence,
    /// `parse(serialize(d)) == d` and re-serialization is byte-stable (QS7).
    #[test]
    fn operation_generated_diagrams_round_trip(ops in prop::collection::vec(op_strategy(), 1..40)) {
        let d = diagram_model::testgen::replay(&ops);
        let text = ad_format::serialize(&d);
        let parsed = ad_format::parse(&text)
            .unwrap_or_else(|err| panic!("canonical serialization must parse: {err}\n{text}"));
        // Byte-stable re-save (QS7).
        prop_assert_eq!(ad_format::serialize(&parsed), text);
        // Model equality covers objects, order, geometry, labels, anchors, tips and colors.
        prop_assert_eq!(parsed, d);
    }
}

/// S-020 AC3 — scheduled heavy check: the same lossless round-trip property as
/// [`round_trip_is_lossless`] at 10× proptest's default iteration count (256 → 2560
/// cases). Runs in ordinary `cargo test` on every target.
#[test]
fn round_trip_heavy_2560_cases() {
    use proptest::test_runner::{Config, TestRunner};

    let mut runner = TestRunner::new(Config {
        cases: 2560,
        max_shrink_iters: 10_000,
        ..Config::default()
    });
    runner
        .run(&diagram_strategy(), |d: Diagram| {
            let text = ad_format::serialize(&d);
            let parsed = ad_format::parse(&text)
                .unwrap_or_else(|err| panic!("canonical serialization must parse: {err}\n{text}"));
            assert_eq!(
                ad_format::serialize(&parsed),
                text,
                "re-save must be byte-stable"
            );
            assert_eq!(parsed, d, "round-trip must be lossless");
            Ok(())
        })
        .expect("lossless round-trip holds at 2560 cases");
}
