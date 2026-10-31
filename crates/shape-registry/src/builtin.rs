//! The predefined shape catalog (FR11): every built-in shape is plain data — a glyph
//! pattern plus its standard default anchors, exactly as user-defined shapes from
//! `.adc` collections. Built-ins are named by bare type-id; custom shapes use
//! `collection/name`.
//!
//! Standard default anchors ([shape-template](../../../doc/arc42/12-concepts/shape-template.md)):
//! the midpoints of the four sides — `n`, `s`, `w`, `e` (center point for circles).

use crate::geometry::CellPos;
use crate::types::{AnchorSet, GeometryKind, ShapeType, TemplateGrid};

/// The built-in catalog: box and ellipse/circle, in deterministic order.
pub const BUILTIN_NAMES: [&str; 2] = ["box", "ellipse"];

/// Builds a built-in shape type from its glyph rows and standard default anchors.
fn builtin(
    name: &str,
    geometry: GeometryKind,
    rows: &[&str],
    n: (u32, u32),
    s: (u32, u32),
    w: (u32, u32),
    e: (u32, u32),
) -> ShapeType {
    let glyph_pattern = TemplateGrid::from_rows(rows).expect("built-in glyphs are valid");
    let mut anchor_set = AnchorSet::new();
    for (anchor_name, pos) in [
        ("n", CellPos::new(n.0, n.1)),
        ("s", CellPos::new(s.0, s.1)),
        ("w", CellPos::new(w.0, w.1)),
        ("e", CellPos::new(e.0, e.1)),
    ] {
        anchor_set
            .add(anchor_name, pos)
            .expect("standard anchors are distinct");
    }
    ShapeType {
        type_id: name.to_owned(),
        name: name.to_owned(),
        geometry,
        glyph_pattern,
        anchor_set,
        resize_rules: None, // v1: no resize rules (S-005 AC4)
    }
}

/// The built-in box: a rectangle with mid-side anchors.
///
/// `Rect` geometry: borders are resolved from the instance's own pos/size (any
/// size), so instances of different sizes join cleanly. The 9×5 art below
/// documents the natural appearance; it is not stamped for Rect shapes.
fn box_shape() -> ShapeType {
    builtin(
        "box",
        GeometryKind::Rect,
        &[
            "+-------+",
            "|       |",
            "|       |",
            "|       |",
            "+-------+",
        ],
        (4, 0), // n: top edge midpoint
        (4, 4), // s: bottom edge midpoint
        (0, 2), // w: left edge midpoint
        (8, 2), // e: right edge midpoint
    )
}

/// The built-in ellipse/circle: a 9×5 svgbob-style oval with mid-side anchors.
///
/// `Ellipse` geometry: the outline is re-rasterized deterministically at the
/// instance's own pos/size (the art below documents the natural appearance), so
/// the shape is resizable like `box` (FR1, S-013 AC1; B-001, ADR 013 v3).
fn ellipse_shape() -> ShapeType {
    builtin(
        "ellipse",
        GeometryKind::Ellipse,
        &[
            "   ___   ",
            " ,'   '. ",
            "(       )",
            " `.   .' ",
            "   `-'   ",
        ],
        (4, 0), // n: top center
        (4, 4), // s: bottom center
        (0, 2), // w: leftmost cell of the middle row
        (8, 2), // e: rightmost cell of the middle row
    )
}

/// The full built-in catalog as data.
#[must_use]
pub fn builtin_catalog() -> Vec<ShapeType> {
    vec![box_shape(), ellipse_shape()]
}

#[cfg(test)]
mod tests {
    use super::builtin_catalog;
    use crate::geometry::CellPos;
    use crate::types::GeometryKind;

    /// AC1: the catalog contains at least box and circle/ellipse, defined as data.
    #[test]
    fn catalog_contains_box_and_ellipse() {
        let catalog = builtin_catalog();
        let names: Vec<&str> = catalog.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"box"));
        assert!(names.contains(&"ellipse"));
    }

    /// S-006: geometry kinds — box is Rect (borders from instance pos/size),
    /// ellipse is Ellipse (re-rasterized at the instance extent, B-001).
    #[test]
    fn builtins_carry_their_geometry_kind() {
        let catalog = builtin_catalog();
        let box_shape = catalog
            .iter()
            .find(|s| s.name == "box")
            .expect("box is built in");
        assert_eq!(box_shape.geometry, GeometryKind::Rect);
        let ellipse = catalog
            .iter()
            .find(|s| s.name == "ellipse")
            .expect("ellipse is built in");
        assert_eq!(ellipse.geometry, GeometryKind::Ellipse);
    }

    /// AC1/AC4: every built-in carries exactly the four standard default anchors and no resize rules.
    #[test]
    fn every_builtin_has_standard_anchors_and_no_resize_rules() {
        for shape in builtin_catalog() {
            assert_eq!(
                shape.resize_rules, None,
                "{:?} must have no resize rules in v1",
                shape.name
            );
            let names: Vec<&String> = shape.anchor_set.iter().map(|(name, _)| name).collect();
            // Alphabetical (deterministic) iteration order of the anchor set.
            assert_eq!(
                names,
                vec!["e", "n", "s", "w"],
                "{} has the four standard anchors",
                shape.name
            );
        }
    }

    /// AC1: the box's exact anchor set — midpoints of its 9×5 sides.
    #[test]
    fn box_anchor_set_is_the_mid_side_points() {
        let rect = builtin_catalog()
            .into_iter()
            .find(|s| s.name == "box")
            .expect("box is built in");
        assert_eq!(rect.size().width, 9);
        assert_eq!(rect.size().height, 5);
        assert_eq!(rect.anchor_set.get("n"), Some(CellPos::new(4, 0)));
        assert_eq!(rect.anchor_set.get("s"), Some(CellPos::new(4, 4)));
        assert_eq!(rect.anchor_set.get("w"), Some(CellPos::new(0, 2)));
        assert_eq!(rect.anchor_set.get("e"), Some(CellPos::new(8, 2)));
        // Anchor cells sit on the border and are never empty.
        for (name, pos) in [
            ("n", CellPos::new(4, 0)),
            ("s", CellPos::new(4, 4)),
            ("w", CellPos::new(0, 2)),
            ("e", CellPos::new(8, 2)),
        ] {
            let cell = rect.glyph_pattern.get(pos).expect("anchor in bounds");
            assert_ne!(cell, ' ', "anchor '{name}' sits on a drawn cell");
        }
    }

    /// AC1: the ellipse's exact anchor set — mid-side points of its 9×5 oval.
    #[test]
    fn ellipse_anchor_set_is_the_mid_side_points() {
        let catalog = builtin_catalog();
        let ellipse = catalog
            .into_iter()
            .find(|s| s.name == "ellipse")
            .expect("ellipse is built in");
        assert_eq!(ellipse.size().width, 9);
        assert_eq!(ellipse.size().height, 5);
        assert_eq!(ellipse.anchor_set.get("n"), Some(CellPos::new(4, 0)));
        assert_eq!(ellipse.anchor_set.get("s"), Some(CellPos::new(4, 4)));
        assert_eq!(ellipse.anchor_set.get("w"), Some(CellPos::new(0, 2)));
        assert_eq!(ellipse.anchor_set.get("e"), Some(CellPos::new(8, 2)));
        // The middle row is the widest part of the oval; both side anchors sit on it.
        assert_eq!(ellipse.glyph_pattern.row(2), Some("(       )".to_owned()));
    }

    /// Built-in glyphs use only printable ASCII (the svgbob-alphabet subset, ADR 001/005).
    #[test]
    fn built_in_glyphs_are_printable_ascii() {
        for shape in builtin_catalog() {
            let rows: Vec<String> = (0..shape.glyph_pattern.height())
                .map(|y| shape.glyph_pattern.row(y).expect("row in bounds"))
                .collect();
            for row in &rows {
                assert!(row.chars().all(|c| matches!(c as u32, 0x20..=0x7e)));
            }
        }
    }
}
