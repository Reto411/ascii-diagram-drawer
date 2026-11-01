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
/// The minor version needed by `resize` (C10) and `region` (FR19) statements;
/// collections without either serialize at the lower 1.0 baseline.
const MINOR_WITH_RESIZE: u32 = 1;

/// Serializes `collection` to canonical `.adc` text (trailing newline included).
///
/// Layout per the normative grammar in
/// [shape-template](../../../doc/arc42/12-concepts/shape-template.md):
/// `adc <MAJOR>.<MINOR>` header, then one block per template — declaration line,
/// exactly `height` grid rows (trailing spaces trimmed), then its anchor statements
/// in name order and its `resize` statements in edge order (top, bottom, left, right).
///
/// The header minor is the lowest version that can express the collection: 1.0 when
/// no template carries resize rules, 1.1 otherwise. Rule sets serialize their base
/// character per edge (the file syntax declares one character serving as both insert
/// and remove); parity overrides are emitted as `even`/`odd` suffixes.
pub fn serialize_collection(collection: &Collection) -> String {
    // A collection with any resize rules or text regions serializes at 1.1 — the
    // reader of 1.0 files would reject either statement as unknown.
    let minor = if collection
        .types
        .iter()
        .any(|t| t.resize_rules.is_some() || !t.regions.is_empty())
    {
        MINOR_WITH_RESIZE
    } else {
        0
    };
    let mut out = format!("adc {MAJOR}.{minor}\n");
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
        // FR19: regions by name between anchors and resize rules.
        for (name, rect) in template.regions.iter() {
            out.push_str(&format!(
                "region {name} at {} {} w {} h {}\n",
                rect.x, rect.y, rect.width, rect.height
            ));
        }
        if let Some(rules) = &template.resize_rules {
            // Canonical order (C10): edges top, bottom, left, right; per edge the base
            // rule first, then its even and odd parity overrides when declared.
            for (edge, rule) in [
                ("top", rules.top),
                ("bottom", rules.bottom),
                ("left", rules.left),
                ("right", rules.right),
            ] {
                out.push_str(&format!(
                    "resize {edge} insert \"{}\" remove \"{}\"\n",
                    rule.insert, rule.remove
                ));
                if let Some(parity) = &rule.even {
                    out.push_str(&format!(
                        "resize {edge} even insert \"{}\" remove \"{}\"\n",
                        parity.insert, parity.remove
                    ));
                }
                if let Some(parity) = &rule.odd {
                    out.push_str(&format!(
                        "resize {edge} odd insert \"{}\" remove \"{}\"\n",
                        parity.insert, parity.remove
                    ));
                }
            }
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
            regions: crate::types::RegionSet::new(),
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

    /// AC2 (S-024): a template with resize rules serializes its statements in edge
    /// order after the anchors (base, then even, then odd per edge), and the
    /// collection's header bumps to 1.1.
    #[test]
    fn resize_rules_serialize_in_edge_order_and_bump_the_header() {
        let rules = crate::types::ResizeRules {
            top: crate::types::EdgeRule {
                insert: '-',
                remove: '-',
                even: Some(crate::types::ParityChars {
                    insert: '=',
                    remove: '=',
                }),
                odd: None,
            },
            bottom: crate::types::EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            left: crate::types::EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
            right: crate::types::EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
        };
        let t = ShapeType {
            type_id: "c/box".to_owned(),
            name: "box".to_owned(),
            glyph_pattern: TemplateGrid::from_rows(&["+--+", "|  |", "+--+"]).expect("valid"),
            anchor_set: AnchorSet::new(),
            regions: crate::types::RegionSet::new(),
            geometry: GeometryKind::Pattern,
            resize_rules: Some(rules),
        };
        let collection = Collection {
            name: "c".to_owned(),
            types: vec![t],
        };
        assert_eq!(
            serialize_collection(&collection),
            "adc 1.1\ntemplate box 4 x 3\n+--+\n|  |\n+--+\nresize top insert \"-\" remove \"-\"\nresize top even insert \"=\" remove \"=\"\nresize bottom insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"|\"\n"
        );
    }

    /// AC2 (S-024): round trip — a collection with rules re-parses and re-serializes
    /// byte-stable in canonical order, even when the input file lists statements in a
    /// different order; a rule-free collection stays at header 1.0.
    #[test]
    fn resize_round_trip_is_stable() {
        // Input: non-canonical statement order (right before left, odd before base).
        let text = "adc 1.1\ntemplate box 4 x 3\n+--+\n|  |\n+--+\nanchor n at 2 0\nresize right odd insert \"!\" remove \"!\"\nresize top insert \"-\" remove \"-\"\nresize top even insert \"=\" remove \"=\"\nresize bottom insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"!\"\n";
        let parsed = parse_collection("c", text).expect("parses");
        assert!(parsed.collection.types[0].resize_rules.is_some());
        // Canonical re-save: fixed edge order, base before parity overrides.
        let reser = serialize_collection(&parsed.collection);
        assert_eq!(
            reser,
            "adc 1.1\ntemplate box 4 x 3\n+--+\n|  |\n+--+\nanchor n at 2 0\nresize top insert \"-\" remove \"-\"\nresize top even insert \"=\" remove \"=\"\nresize bottom insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"!\"\nresize right odd insert \"!\" remove \"!\"\n"
        );
        // And the canonical form round-trips through the parser unchanged.
        let reparsed = parse_collection("c", &reser).expect("re-parses");
        assert_eq!(serialize_collection(&reparsed.collection), reser);

        // A rule-free collection keeps the 1.0 baseline header.
        let plain = parse_collection("c", "adc 1.0\ntemplate t 2 x 2\nab\ncd\n").expect("parses");
        assert!(serialize_collection(&plain.collection).starts_with("adc 1.0\n"));
    }

    /// AC1 (S-037): regions serialize between anchors and resize rules, in name order,
    /// bump the header to 1.1, and round-trip unchanged.
    #[test]
    fn region_round_trip_is_stable() {
        let text = "adc 1.1\ntemplate box 6 x 4\n+----+\n|    |\n|    |\n+----+\nanchor n at 3 0\nregion body at 1 2 w 4 h 1\nregion title at 1 1 w 4 h 1\nresize top insert \"-\" remove \"-\"\nresize bottom insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"|\"\n";
        let parsed = parse_collection("c", text).expect("parses");
        // Canonical re-save: regions by name (body before title), after anchors,
        // before resize rules; header at 1.1.
        let reser = serialize_collection(&parsed.collection);
        assert_eq!(
            reser,
            "adc 1.1\ntemplate box 6 x 4\n+----+\n|    |\n|    |\n+----+\nanchor n at 3 0\nregion body at 1 2 w 4 h 1\nregion title at 1 1 w 4 h 1\nresize top insert \"-\" remove \"-\"\nresize bottom insert \"-\" remove \"-\"\nresize left insert \"|\" remove \"|\"\nresize right insert \"|\" remove \"|\"\n"
        );
        let reparsed = parse_collection("c", &reser).expect("re-parses");
        assert_eq!(serialize_collection(&reparsed.collection), reser);

        // Regions alone (no resize rules) still bump the header to 1.1.
        let regions_only = parse_collection(
            "c",
            "adc 1.1\ntemplate t 4 x 3\n+--+\n|  |\n+--+\nregion r at 1 1 w 2 h 1\n",
        )
        .expect("parses");
        assert!(serialize_collection(&regions_only.collection).starts_with("adc 1.1\n"));
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
