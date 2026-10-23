//! Canonical serializer: [`Diagram`] / [`Document`] → `.ad` text, per the canonical-form rules in
//! [diagram-file-format.md](../../../doc/arc42/12-concepts/diagram-file-format.md).
//!
//! Deterministic byte-stable output (QS7): header first, then `scale`, then objects in stored
//! order; exactly one space between tokens; LF line endings; no comments, blank lines or
//! trailing whitespace.

use diagram_model::Diagram;
use diagram_model::objects::{ConnectorStyle, Object, ShapeInstance, TextInstance};

use crate::parser::{Document, Slot};

/// The version emitted by this writer.
const VERSION: &str = "ad 1.0";

/// Serializes a [`Diagram`] to canonical `.ad` text.
#[must_use]
pub fn serialize(diagram: &Diagram) -> String {
    let mut out = String::new();
    out.push_str(VERSION);
    out.push('\n');
    push_scale(&mut out, diagram.scale());
    for object in diagram.objects() {
        push_object(&mut out, object);
    }
    out
}

/// Serializes a parsed [`Document`], re-emitting preserved unknown statements at their
/// original positions and the source file's version, so the result parses back losslessly
/// (forward-compat round trip, QS7 across minor upgrades).
#[must_use]
pub fn serialize_document(document: &Document) -> String {
    let mut out = String::new();
    push_version(&mut out, document.version.0, document.version.1);
    out.push('\n');
    push_scale(&mut out, document.diagram.scale());
    for slot in &document.slots {
        match slot {
            Slot::Object(index) => {
                let object = &document.diagram.objects()[*index];
                push_object(&mut out, object);
            }
            Slot::Unknown(raw_line) => {
                out.push_str(raw_line.trim());
                out.push('\n');
            }
        }
    }
    out
}

/// Emits a canonical `ad <major>.<minor>` header line (without the newline).
fn push_version(out: &mut String, major: u32, minor: u32) {
    out.push_str("ad ");
    push_int(out, major);
    out.push('.');
    push_int(out, minor);
}

/// Emits the always-present `scale` statement.
fn push_scale(out: &mut String, scale: u32) {
    out.push_str("scale ");
    out.push_str(&scale.to_string());
    out.push('\n');
}

/// Emits one object statement in canonical form.
fn push_object(out: &mut String, object: &Object) {
    match object {
        Object::Shape(shape) => push_shape(out, shape),
        Object::Text(text) => push_text(out, text),
        Object::Connector(connector) => {
            out.push_str("connect as ");
            out.push_str(connector.id.as_str());
            out.push_str(" from ");
            push_anchor_ref(
                out,
                connector.from.object().as_str(),
                connector.from.anchor(),
            );
            out.push_str(" to ");
            push_anchor_ref(out, connector.to.object().as_str(), connector.to.anchor());
            if connector.style == ConnectorStyle::Dashed {
                out.push_str(" dashed");
            }
            out.push('\n');
        }
    }
}

/// Emits `shape <type-id> "<label>" as <id> at <x> <y> w <width> h <height>`.
fn push_shape(out: &mut String, shape: &ShapeInstance) {
    out.push_str("shape ");
    out.push_str(&shape.type_id);
    out.push_str(" \"");
    out.push_str(&escape(&shape.label));
    out.push_str("\" as ");
    out.push_str(shape.id.as_str());
    out.push_str(" at ");
    push_int(out, shape.pos.x);
    out.push(' ');
    push_int(out, shape.pos.y);
    out.push_str(" w ");
    push_int(out, shape.size.width);
    out.push_str(" h ");
    push_int(out, shape.size.height);
    out.push('\n');
}

/// Emits `text as <id> "<content>" at <x> <y>`.
fn push_text(out: &mut String, text: &TextInstance) {
    out.push_str("text as ");
    out.push_str(text.id.as_str());
    out.push_str(" \"");
    out.push_str(&escape(&text.text));
    out.push_str("\" at ");
    push_int(out, text.pos.x);
    out.push(' ');
    push_int(out, text.pos.y);
    out.push('\n');
}

/// Emits `<id>.<anchor>`.
fn push_anchor_ref(out: &mut String, object_id: &str, anchor: &str) {
    out.push_str(object_id);
    out.push('.');
    out.push_str(anchor);
}

/// Emits a canonical decimal integer (u32 never needs leading zeros).
fn push_int(out: &mut String, value: u32) {
    out.push_str(&value.to_string());
}

/// Escapes a string for the v1 grammar: only `\"`, `\\` and `\n`.
#[must_use]
fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        reason = "test code: building expected values"
    )]

    use super::*;
    use crate::parser::parse_document;
    use diagram_model::anchors::AnchorRef;
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::{ConnectorInstance, ObjectId, TextInstance};

    /// Builds a small multi-object diagram: shape + text + connector, scale 2.
    fn sample_diagram() -> Diagram {
        let mut diagram = Diagram::new();
        let s1 = ObjectId::new("s1").expect("valid id");
        let size = CellSize::new(8, 4).expect("valid size");
        diagram
            .place_shape(
                ShapeInstance::new(s1.clone(), "box", "Server A", CellPos::new(1, 1), size)
                    .expect("valid shape"),
            )
            .expect("place");
        diagram
            .place_text(
                TextInstance::new(
                    ObjectId::new("t1").expect("valid id"),
                    CellPos::new(3, 2),
                    "hi",
                )
                .expect("valid text"),
            )
            .expect("place");
        let from = AnchorRef::new(s1.clone(), "e").expect("valid anchor");
        let to = AnchorRef::new(s1.clone(), "w").expect("valid anchor");
        diagram
            .connect(ConnectorInstance::new(
                ObjectId::new("c1").expect("valid id"),
                from,
                to,
                ConnectorStyle::Solid,
            ))
            .expect("connect");
        diagram.set_scale(2).expect("scale");
        diagram
    }

    #[test]
    fn empty_diagram_serializes_to_header_and_default_scale() {
        let diagram = Diagram::new();
        assert_eq!(serialize(&diagram), "ad 1.0\nscale 1\n");
    }

    #[test]
    fn sample_diagram_matches_expected_canonical_text() {
        let out = serialize(&sample_diagram());
        let expected = "ad 1.0\n\
            scale 2\n\
            shape box \"Server A\" as s1 at 1 1 w 8 h 4\n\
            text as t1 \"hi\" at 3 2\n\
            connect as c1 from s1.e to s1.w\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn dashed_style_is_emitted_only_when_non_default() {
        let mut diagram = Diagram::new();
        let s1 = ObjectId::new("s1").expect("valid id");
        diagram
            .place_shape(
                ShapeInstance::new(s1.clone(), "box", "", CellPos::origin(), CellSize::unit())
                    .expect("valid"),
            )
            .expect("place");
        let from = AnchorRef::new(s1.clone(), "e").expect("valid anchor");
        let to = AnchorRef::new(s1, "w").expect("valid anchor");
        diagram
            .connect(ConnectorInstance::new(
                ObjectId::new("c1").expect("valid id"),
                from,
                to,
                ConnectorStyle::Dashed,
            ))
            .expect("connect");
        assert!(serialize(&diagram).contains(" dashed\n"));
    }

    #[test]
    fn strings_are_escaped_minimally() {
        let mut diagram = Diagram::new();
        diagram
            .place_text(
                TextInstance::new(
                    ObjectId::new("t1").expect("valid id"),
                    CellPos::origin(),
                    "a\"b\\c\nd",
                )
                .expect("valid text"),
            )
            .expect("place");
        assert_eq!(
            serialize(&diagram),
            "ad 1.0\nscale 1\ntext as t1 \"a\\\"b\\\\c\\nd\" at 0 0\n"
        );
    }

    #[test]
    fn other_characters_pass_through_unescaped() {
        let mut diagram = Diagram::new();
        diagram
            .place_text(
                TextInstance::new(
                    ObjectId::new("t1").expect("valid id"),
                    CellPos::origin(),
                    "tab\there # not a comment",
                )
                .expect("valid text"),
            )
            .expect("place");
        assert_eq!(
            serialize(&diagram),
            "ad 1.0\nscale 1\ntext as t1 \"tab\there # not a comment\" at 0 0\n"
        );
    }

    #[test]
    fn document_round_trip_keeps_unknowns_in_place() {
        let input = "ad 1.3\ntext as t1 \"x\" at 0 0\nfoo one\ntext as t2 \"y\" at 1 1\n";
        let doc = parse_document(input).expect("valid");
        let out = serialize_document(&doc);
        // The source version (1.3) is preserved: it is what makes `foo` legal.
        assert_eq!(
            out,
            "ad 1.3\nscale 1\ntext as t1 \"x\" at 0 0\nfoo one\ntext as t2 \"y\" at 1 1\n"
        );
        // And the re-saved file parses back to the same document (QS7 across minor upgrades).
        assert_eq!(parse_document(&out).expect("re-saves losslessly"), doc);
    }

    #[test]
    fn document_without_unknowns_matches_plain_serialize() {
        let input = "ad 1.0\nscale 2\ntext as t1 \"x\" at 0 0\n";
        let doc = parse_document(input).expect("valid");
        assert_eq!(serialize_document(&doc), serialize(&doc.diagram));
    }

    #[test]
    fn output_never_contains_crlf_or_trailing_whitespace() {
        let out = serialize(&sample_diagram());
        assert!(!out.contains('\r'));
        for line in out.lines() {
            assert_eq!(line, line.trim_end(), "trailing whitespace in: {line:?}");
        }
    }

    #[test]
    fn output_is_idempotent_through_parse() {
        let once = serialize(&sample_diagram());
        let twice = crate::parser::parse(&once).map(|d| serialize(&d));
        assert_eq!(twice, Ok(once));
    }
}
