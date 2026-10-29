//! Canonical serializer: [`Collection`] → `.adc` file text, the inverse of
//! [`parse_collection`](crate::templates::parse_collection) for files this reader
//! understands. Landed with S-017 (the template editor persists saved templates).
//!
//! Output is deterministic — the same collection always serializes to the same bytes:
//! templates in declaration order, grid rows with trailing spaces trimmed (the parser
//! right-pads them back), anchors in name order. `parse(serialize(c)) == c` for every
//! well-formed `c`; the golden tests pin the exact bytes.

use crate::types::Collection;

#[cfg(test)]
use crate::types::ShapeType;

/// The major format version this writer emits.
const MAJOR: u32 = 1;
/// The minor version this writer emits.
const MINOR: u32 = 0;

/// Serializes `collection` to canonical `.adc` text (trailing newline included).
///
/// Layout per the normative grammar in
/// [shape-template](../../../doc/arc42/12-concepts/shape-template.md):
/// `adc <MAJOR>.<MINOR>` header, then one block per template — declaration line,
/// exactly `height` grid rows (trailing spaces trimmed), then its anchor statements
/// in name order.
pub fn serialize_collection(collection: &Collection) -> String {
    let mut out = format!("adc {MAJOR}.{MINOR}\n");
    for template in &collection.types {
        out.push_str(&format!(
            "template {} {} x {}\n",
            template.name,
            template.glyph_pattern.width(),
            template.glyph_pattern.height()
        ));
        for y in 0..template.glyph_pattern.height() {
            let row = template
                .glyph_pattern
                .row(y)
                .expect("row index built from the grid's own height");
            // Trim trailing spaces: the canonical form never emits them, and the
            // parser right-pads rows back to the declared width.
            out.push_str(row.trim_end_matches(' '));
            out.push('\n');
        }
        for (name, pos) in template.anchor_set.iter() {
            out.push_str(&format!("anchor {name} at {} {}\n", pos.x, pos.y));
        }
    }
    out
}

/// Serializes a single template block (no header) — used by tests to pin the
/// per-template layout.
#[cfg(test)]
pub(crate) fn serialize_template(template: &ShapeType) -> String {
    let mut out = String::new();
    for line in serialize_collection(&Collection {
        name: String::new(),
        types: vec![template.clone()],
    })
    .lines()
    {
        if line.starts_with("adc ") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{serialize_collection, serialize_template};
    use crate::error::GridError;
    use crate::templates::parse_collection;
    use crate::types::{AnchorSet, Collection, GeometryKind, ShapeType, TemplateGrid};

    fn template(
        collection: &str,
        name: &str,
        rows: &[&str],
        anchors: &[(&str, u32, u32)],
    ) -> ShapeType {
        let mut anchor_set = AnchorSet::new();
        for (anchor, x, y) in anchors {
            anchor_set
                .add(anchor, crate::geometry::CellPos::new(*x, *y))
                .unwrap();
        }
        ShapeType {
            type_id: format!("{collection}/{name}"),
            name: name.to_owned(),
            glyph_pattern: TemplateGrid::from_rows(rows).expect("valid rows"),
            anchor_set,
            geometry: GeometryKind::Pattern,
            resize_rules: None,
        }
    }

    /// AC3 (S-017): the exact canonical bytes — header, declaration, trimmed rows,
    /// anchors in name order.
    #[test]
    fn serializes_to_the_exact_canonical_bytes() {
        let collection = Collection {
            name: "stars".to_owned(),
            types: vec![template(
                "stars",
                "star",
                &["  *  ", " *** ", "* * *"],
                &[("e", 5, 1), ("n", 2, 0)],
            )],
        };
        assert_eq!(
            serialize_collection(&collection),
            "adc 1.0\ntemplate star 5 x 3\n  *\n ***\n* * *\nanchor e at 5 1\nanchor n at 2 0\n"
        );
    }

    /// Trailing spaces are trimmed per row; interior and leading spaces survive.
    #[test]
    fn trailing_row_spaces_are_trimmed() {
        let t = template("c", "pad", &["ab  ", "abcd"], &[("w", 0, 1)]);
        assert_eq!(
            serialize_template(&t),
            "template pad 4 x 2\nab\nabcd\nanchor w at 0 1\n"
        );
    }

    /// AC3 (S-017): round trip — parse(serialize(c)) reproduces the collection exactly.
    #[test]
    fn round_trip_preserves_the_collection() {
        let collection = Collection {
            name: "mixed".to_owned(),
            types: vec![
                template("mixed", "a", &["x"], &[("only", 0, 0)]),
                template(
                    "mixed",
                    "b",
                    &["----", "|  |", "----"],
                    &[("s", 2, 2), ("n", 2, 0)],
                ),
            ],
        };
        let text = serialize_collection(&collection);
        let reparsed = parse_collection("mixed", &text).expect("canonical output parses");
        assert_eq!(reparsed.collection, collection);
        // And the re-serialization is byte-stable (idempotent canonical form).
        assert_eq!(serialize_collection(&reparsed.collection), text);
    }

    /// Multiple templates keep declaration order; a template without anchors emits
    /// only its grid.
    #[test]
    fn declaration_order_and_anchorless_templates() {
        let collection = Collection {
            name: "c".to_owned(),
            types: vec![
                template("c", "second", &["q"], &[]),
                template("c", "first", &["z"], &[("n", 0, 0)]),
            ],
        };
        assert_eq!(
            serialize_collection(&collection),
            "adc 1.0\ntemplate second 1 x 1\nq\ntemplate first 1 x 1\nz\nanchor n at 0 0\n"
        );
    }

    /// Grid construction rejects the same things the parser does — the serializer
    /// never sees an invalid grid (TemplateGrid invariants).
    #[test]
    fn grid_invariants_are_enforced_at_construction() {
        assert_eq!(TemplateGrid::from_rows(&[]), Err(GridError::Empty));
        assert_eq!(
            TemplateGrid::from_rows(&["ab", "c"]),
            Err(GridError::RaggedRows)
        );
    }
}
