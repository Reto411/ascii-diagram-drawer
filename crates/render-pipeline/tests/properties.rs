//! Property tests (S-006 AC1/AC4): the pipeline is pure and deterministic, and every
//! emitted character stays inside svgbob's ASCII input alphabet (ADR 001 v2).
//!
//! Purity reading: svgbob consumes printable ASCII; characters outside its *drawing*
//! vocabulary degrade to literal text (ADR 001 consequences). So the invariant is:
//! every cell is a space or printable ASCII, and every non-alphanumeric character is a
//! structural/quote glyph of the svgbob alphabet. Labels are generated from a charset
//! that mixes plain letters/digits with drawing glyphs to exercise quote escaping.

use diagram_model::Diagram;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{Object, ObjectId, ShapeInstance, TextInstance};
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

/// A random diagram: up to 6 objects inside a 24x16 canvas, scale 1..3.
fn random_diagram() -> impl Strategy<Value = Diagram> {
    (
        one_object(0),
        one_object(1),
        one_object(2),
        one_object(3),
        one_object(4),
        one_object(5),
    )
        .prop_map(|slots| {
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
            for obj in objects {
                match obj {
                    Object::Shape(s) => d.place_shape(s).expect("unique ids"),
                    Object::Text(t) => d.place_text(t).expect("unique ids"),
                    Object::Connector(_) => unreachable!(),
                }
            }
            d
        })
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
}
