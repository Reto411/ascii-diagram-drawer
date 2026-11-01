//! The predefined shape catalog (FR11/FR18): every built-in shape is plain data — a
//! geometry kind, anchor set, and optional glyph art — so the registry, the pipeline,
//! and the GUI all consume one source of truth.

use crate::geometry::{CellPos, Region};
use crate::types::{AnchorSet, GeometryKind, RegionSet, ShapeType, TemplateGrid};

/// The built-in catalog in deterministic order: box and ellipse (S-005) first, then the
/// core UML set (FR18 / CR-015) in §14's list order.
pub const BUILTIN_NAMES: [&str; 13] = [
    "box",
    "ellipse",
    "lollipop",
    "actor",
    "activation",
    "component",
    "port",
    "deployment",
    "artifact",
    "state",
    "decision",
    "initial",
    "final",
];

/// The built-in catalog in deterministic order.
#[must_use]
pub fn builtin_catalog() -> Vec<ShapeType> {
    vec![
        box_shape(),
        ellipse_shape(),
        lollipop_shape(),
        actor_shape(),
        activation_shape(),
        component_shape(),
        port_shape(),
        deployment_shape(),
        artifact_shape(),
        state_shape(),
        decision_shape(),
        initial_shape(),
        final_shape(),
    ]
}

/// The four standard edge-midpoint anchors (FR11) every built-in declares.
fn standard_anchors(n: (u32, u32), s: (u32, u32), w: (u32, u32), e: (u32, u32)) -> AnchorSet {
    let mut anchors = AnchorSet::new();
    for (name, pos) in [("n", n), ("s", s), ("w", w), ("e", e)] {
        anchors
            .add(name, CellPos::new(pos.0, pos.1))
            .expect("built-in anchor is valid");
    }
    anchors
}

fn builtin(
    name: &str,
    geometry: GeometryKind,
    rows: &[&str],
    n: (u32, u32),
    s: (u32, u32),
    w: (u32, u32),
    e: (u32, u32),
) -> ShapeType {
    let anchors = standard_anchors(n, s, w, e);
    ShapeType {
        type_id: name.into(),
        name: name.to_string(),
        glyph_pattern: TemplateGrid::from_rows(rows).expect("built-in art is valid"),
        anchor_set: anchors,
        geometry,
        resize_rules: None,
        regions: RegionSet::new(),
    }
}

/// A Pattern-geometry UML built-in (FR18) with the standard `n`/`s`/`w`/`e` anchors and,
/// for compartmented shapes, a single `name` region covering the interior so its UML
/// semantics are written through S-037's shape-owned text boxes.
fn uml_shape(
    name: &str,
    rows: &[&str],
    n: (u32, u32),
    s: (u32, u32),
    w: (u32, u32),
    e: (u32, u32),
    name_region: Option<Region>,
) -> ShapeType {
    let anchors = standard_anchors(n, s, w, e);
    let mut regions = RegionSet::new();
    if let Some(region) = name_region {
        regions
            .add("name", region)
            .expect("built-in region is valid");
    }
    ShapeType {
        type_id: name.into(),
        name: name.to_string(),
        glyph_pattern: TemplateGrid::from_rows(rows).expect("built-in art is valid"),
        anchor_set: anchors,
        geometry: GeometryKind::Pattern,
        resize_rules: None,
        regions,
    }
}

/// A rectangle with edge-midpoint anchors.
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
        (4, 0),
        (4, 4),
        (0, 2),
        (8, 2),
    )
}

/// An ellipse with edge-midpoint anchors.
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
        (4, 0),
        (4, 4),
        (0, 2),
        (8, 2),
    )
}

/// UML interface lollipop: a small hollow circle on a stick.
fn lollipop_shape() -> ShapeType {
    uml_shape(
        "lollipop",
        &[" _ ", "( )", "`-'", " | ", " | ", " | ", " | "],
        (1, 0),
        (1, 6),
        (0, 1),
        (2, 1),
        None,
    )
}

/// UML actor: the stick figure.
fn actor_shape() -> ShapeType {
    uml_shape(
        "actor",
        &["   o   ", "  /|\\  ", "  / \\  ", "  / \\  "],
        (3, 0),
        (3, 3),
        (2, 1),
        (4, 1),
        None,
    )
}

/// UML activation bar: the thin rectangle on a lifeline.
fn activation_shape() -> ShapeType {
    uml_shape(
        "activation",
        &[
            "+---+", "|   |", "|   |", "|   |", "|   |", "|   |", "+---+",
        ],
        (2, 0),
        (2, 6),
        (0, 3),
        (4, 3),
        None,
    )
}

/// UML component: the rectangle with a small attached tab; its body is the `name` region.
fn component_shape() -> ShapeType {
    uml_shape(
        "component",
        &[
            "+--+--------+",
            "|  |        |",
            "+--+        |",
            "   |        |",
            "   +--------+",
        ],
        (7, 0),
        (7, 4),
        (0, 1),
        (12, 2),
        Some(Region::new(4, 1, 8, 3)),
    )
}

/// UML port: the small filled square.
fn port_shape() -> ShapeType {
    uml_shape(
        "port",
        &["*****", "*****", "*****"],
        (2, 0),
        (2, 2),
        (0, 1),
        (4, 1),
        None,
    )
}

/// UML deployment node: the 3-D box; its front face is the `name` region.
fn deployment_shape() -> ShapeType {
    uml_shape(
        "deployment",
        &[
            "  _________",
            " /       / ",
            "+-------+  ",
            "|       |  ",
            "+-------+  ",
        ],
        (6, 0),
        (4, 4),
        (0, 3),
        (8, 3),
        Some(Region::new(1, 3, 7, 1)),
    )
}

/// UML artifact: the document with a folded corner; its body is the `name` region.
fn artifact_shape() -> ShapeType {
    uml_shape(
        "artifact",
        &[
            "+-----   ",
            "|     /  ",
            "|       +",
            "|       |",
            "|       |",
            "+-------+",
        ],
        (3, 0),
        (4, 5),
        (0, 3),
        (8, 3),
        Some(Region::new(1, 1, 7, 4)),
    )
}

/// UML state: the rounded rectangle; its interior is the `name` region.
fn state_shape() -> ShapeType {
    uml_shape(
        "state",
        &[
            " .-------. ",
            " |       | ",
            " |       | ",
            " |       | ",
            " `-------' ",
        ],
        (5, 0),
        (5, 4),
        (1, 2),
        (9, 2),
        Some(Region::new(2, 1, 7, 3)),
    )
}

/// UML decision diamond: its top/bottom points are implied by the art, so `n`/`s` sit on
/// space cells and bind live once a connector attaches.
fn decision_shape() -> ShapeType {
    uml_shape(
        "decision",
        &[
            "   / \\   ",
            "  /   \\  ",
            " /     \\ ",
            "/       \\",
            "\\       /",
            " \\     / ",
            "  \\   /  ",
            "   \\ /   ",
        ],
        (4, 0),
        (4, 7),
        (0, 3),
        (8, 3),
        None,
    )
}

/// UML initial state node: the single filled dot.
fn initial_shape() -> ShapeType {
    uml_shape("initial", &["*"], (0, 0), (0, 0), (0, 0), (0, 0), None)
}

/// UML final state node: the circle with a filled center.
fn final_shape() -> ShapeType {
    uml_shape(
        "final",
        &[" _ ", "(*)", "`-'"],
        (1, 0),
        (1, 2),
        (0, 1),
        (2, 1),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The catalog is exactly the built-in set in deterministic order: box and ellipse
    /// first, then the FR18 core UML list.
    #[test]
    fn catalog_is_deterministic_and_contains_the_core_uml_set() {
        let catalog = builtin_catalog();
        assert_eq!(catalog.len(), 13);
        let names: Vec<&str> = catalog.iter().map(|s| s.type_id.as_str()).collect();
        assert_eq!(names, BUILTIN_NAMES);
        // FR18's minimum list, verbatim.
        for required in [
            "lollipop",
            "actor",
            "activation",
            "component",
            "port",
            "deployment",
            "artifact",
            "state",
            "decision",
            "initial",
            "final",
        ] {
            assert!(
                catalog.iter().any(|s| s.type_id == required),
                "{required} present"
            );
        }
    }

    /// Every built-in declares exactly the four standard anchors (FR11) and no resize
    /// rules: built-ins stamp at their natural size.
    #[test]
    fn every_builtin_has_standard_anchors_and_no_resize_rules() {
        for shape in builtin_catalog() {
            let names: Vec<&str> = shape
                .anchor_set
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            assert_eq!(names, vec!["e", "n", "s", "w"], "{} anchors", shape.type_id);
            assert!(
                shape.resize_rules.is_none(),
                "{} has no resize rules",
                shape.type_id
            );
        }
    }

    /// FR18 — the UML shapes are verbatim glyph patterns, not procedural geometry.
    #[test]
    fn uml_shapes_are_verbatim_patterns() {
        let catalog = builtin_catalog();
        for name in &BUILTIN_NAMES[2..] {
            let shape = catalog
                .iter()
                .find(|s| s.type_id == *name)
                .expect("in catalog");
            assert_eq!(shape.geometry, GeometryKind::Pattern, "{name} is a pattern");
        }
    }

    /// The exact glyph art of every UML shape (the WYSIWYG contract — CR-015 finalized
    /// the art in this story).
    #[test]
    fn uml_shapes_carry_their_exact_glyph_art() {
        let catalog = builtin_catalog();
        let art: &[(&str, &[&str])] = &[
            (
                "lollipop",
                &[" _ ", "( )", "`-'", " | ", " | ", " | ", " | "],
            ),
            ("actor", &["   o   ", "  /|\\  ", "  / \\  ", "  / \\  "]),
            (
                "activation",
                &[
                    "+---+", "|   |", "|   |", "|   |", "|   |", "|   |", "+---+",
                ],
            ),
            (
                "component",
                &[
                    "+--+--------+",
                    "|  |        |",
                    "+--+        |",
                    "   |        |",
                    "   +--------+",
                ],
            ),
            ("port", &["*****", "*****", "*****"]),
            (
                "deployment",
                &[
                    "  _________",
                    " /       / ",
                    "+-------+  ",
                    "|       |  ",
                    "+-------+  ",
                ],
            ),
            (
                "artifact",
                &[
                    "+-----   ",
                    "|     /  ",
                    "|       +",
                    "|       |",
                    "|       |",
                    "+-------+",
                ],
            ),
            (
                "state",
                &[
                    " .-------. ",
                    " |       | ",
                    " |       | ",
                    " |       | ",
                    " `-------' ",
                ],
            ),
            (
                "decision",
                &[
                    "   / \\   ",
                    "  /   \\  ",
                    " /     \\ ",
                    "/       \\",
                    "\\       /",
                    " \\     / ",
                    "  \\   /  ",
                    "   \\ /   ",
                ],
            ),
            ("initial", &["*"]),
            ("final", &[" _ ", "(*)", "`-'"]),
        ];
        for (name, rows) in art {
            let shape = catalog
                .iter()
                .find(|s| s.type_id == *name)
                .expect("in catalog");
            assert_eq!(
                shape.glyph_pattern.width() as usize,
                rows[0].len(),
                "{name} width"
            );
            assert_eq!(
                shape.glyph_pattern.height() as usize,
                rows.len(),
                "{name} height"
            );
            for (y, row) in rows.iter().enumerate() {
                for (x, expected) in row.chars().enumerate() {
                    let actual = shape.glyph_pattern.get(CellPos::new(x as u32, y as u32));
                    assert_eq!(actual, Some(expected), "{name} ({x},{y})");
                }
            }
        }
    }

    /// FR11 — the per-shape anchor positions are pinned; where a midpoint falls on drawn
    /// art the cell is drawn. The deliberate exceptions (decision's implied top/bottom
    /// points, actor's between-feet point) sit on space cells and bind live.
    #[test]
    fn uml_anchor_positions_are_pinned() {
        let catalog = builtin_catalog();
        // name -> (n, s, w, e)
        let anchors: &[(&str, [(u32, u32); 4])] = &[
            ("lollipop", [(1, 0), (1, 6), (0, 1), (2, 1)]),
            ("actor", [(3, 0), (3, 3), (2, 1), (4, 1)]),
            ("activation", [(2, 0), (2, 6), (0, 3), (4, 3)]),
            ("component", [(7, 0), (7, 4), (0, 1), (12, 2)]),
            ("port", [(2, 0), (2, 2), (0, 1), (4, 1)]),
            ("deployment", [(6, 0), (4, 4), (0, 3), (8, 3)]),
            ("artifact", [(3, 0), (4, 5), (0, 3), (8, 3)]),
            ("state", [(5, 0), (5, 4), (1, 2), (9, 2)]),
            ("decision", [(4, 0), (4, 7), (0, 3), (8, 3)]),
            ("initial", [(0, 0); 4]),
            ("final", [(1, 0), (1, 2), (0, 1), (2, 1)]),
        ];
        // Deliberate exceptions: anchors on space cells that bind live once a connector
        // attaches (decision's implied top/bottom points, actor's between-feet point).
        let gaps: &[(&str, &str)] = &[("decision", "n"), ("decision", "s"), ("actor", "s")];
        for (name, positions) in anchors {
            let shape = catalog
                .iter()
                .find(|s| s.type_id == *name)
                .expect("in catalog");
            for ((x, y), anchor_name) in positions.iter().zip(["n", "s", "w", "e"]) {
                let pos = shape.anchor_set.get(anchor_name).expect("anchor exists");
                assert_eq!((pos.x, pos.y), (*x, *y), "{name}.{anchor_name} position");
                let cell = shape.glyph_pattern.get(CellPos::new(*x, *y));
                if gaps.contains(&(*name, anchor_name)) {
                    assert_eq!(cell, Some(' '), "{name}.{anchor_name} is a declared gap");
                } else {
                    assert!(
                        !matches!(cell, None | Some(' ')),
                        "{name}.{anchor_name} sits on drawn art"
                    );
                }
            }
        }
    }

    /// FR19 — the compartmented UML shapes (component, deployment node, artifact, state)
    /// declare exactly one `name` region covering their interior; every other built-in
    /// stays label-only.
    #[test]
    fn compartmented_builtins_declare_exactly_one_name_region() {
        let catalog = builtin_catalog();
        // name -> exact region rect (x, y, width, height)
        type Rect = (u32, u32, u32, u32);
        let expected: &[(&str, Rect)] = &[
            ("component", (4, 1, 8, 3)),
            ("deployment", (1, 3, 7, 1)),
            ("artifact", (1, 1, 7, 4)),
            ("state", (2, 1, 7, 3)),
        ];
        for (name, (x, y, w, h)) in expected {
            let shape = catalog
                .iter()
                .find(|s| s.type_id == *name)
                .expect("in catalog");
            assert_eq!(shape.regions.len(), 1, "{name} has one region");
            let region = shape.regions.get("name").expect("`name` region");
            assert_eq!(
                (region.x, region.y, region.width, region.height),
                (*x, *y, *w, *h),
                "{name} region rect"
            );
        }
        for name in &BUILTIN_NAMES {
            if expected.iter().any(|(n, _)| n == name) {
                continue;
            }
            let shape = catalog
                .iter()
                .find(|s| s.type_id == *name)
                .expect("in catalog");
            assert!(shape.regions.is_empty(), "{name} is label-only");
        }
    }
}
