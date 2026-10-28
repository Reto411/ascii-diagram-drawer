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
    anchors::AnchorRef,
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
type ConnectorSpec = (usize, usize, String, String, bool);

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
    for (i, (a, b, from_anchor, to_anchor, dashed)) in conns.iter().enumerate() {
        if *a >= shapes.len() || *b >= shapes.len() {
            continue; // generated index out of range: skip this connector
        }
        let from = AnchorRef::new(
            id("s", u32::try_from(*a).expect("small index") + 1),
            from_anchor,
        )
        .expect("valid anchor");
        let to = AnchorRef::new(
            id("s", u32::try_from(*b).expect("small index") + 1),
            to_anchor,
        )
        .expect("valid anchor");
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
}
