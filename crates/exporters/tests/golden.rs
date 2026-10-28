//! Golden file tests (S-008 AC1/AC3): every scenario below exports to the exact bytes
//! stored in `tests/golden/<name>.txt` / `<name>.svg`. The SVG goldens pin svgbob's
//! output so an upstream upgrade that changes it is caught here (R4).
//!
//! Regenerate deliberately with `UPDATE_GOLDEN=1 cargo test -p exporters --test golden`
//! and review the diff before committing.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: golden files are trusted fixtures"
)]

use std::fs;
use std::path::Path;

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance};
use render_pipeline::ResolvedGrid;
use shape_registry::ShapeRegistry;

const GOLDEN_DIR: &str = "tests/golden";

fn box_obj(id: u32, label: &str, x: u32, y: u32, w: u32, h: u32) -> Diagram {
    let mut d = Diagram::new();
    d.place_shape(
        ShapeInstance::new(
            ObjectId::generated(id),
            "box",
            label,
            CellPos::new(x, y),
            CellSize::new(w, h).expect("non-zero test size"),
        )
        .expect("valid shape"),
    )
    .expect("place shape");
    d
}

/// One golden scenario: a name plus the exact resolved grid it freezes.
fn scenarios() -> Vec<(&'static str, ResolvedGrid)> {
    let registry = ShapeRegistry::builtin();
    let render = |d: &Diagram| render_pipeline::render(d, &registry).expect("renders");

    // A single unlabeled box: corners and straights only.
    let bare_box = box_obj(0, "", 0, 0, 9, 5);
    // The same box with a label (quote escaping is not triggered — plain letters).
    let box_label = box_obj(0, "Hello", 0, 0, 9, 5);
    // Cross junction: the narrow box's column pierces the big box's border.
    let mut junction = box_obj(0, "", 0, 0, 9, 5);
    junction
        .place_shape(
            ShapeInstance::new(
                ObjectId::generated(1),
                "box",
                "",
                CellPos::new(4, 0),
                CellSize::new(1, 7).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
        .expect("place shape");
    // Connector with arrowhead (ADR 013): straight route, T-junctions, '>' at the target.
    let mut connector = box_obj(0, "A", 0, 0, 9, 5);
    connector
        .place_shape(
            ShapeInstance::new(
                ObjectId::generated(1),
                "box",
                "B",
                CellPos::new(14, 0),
                CellSize::new(9, 5).expect("non-zero test size"),
            )
            .expect("valid shape"),
        )
        .expect("place shape");
    connector
        .connect(ConnectorInstance::new(
            ObjectId::generated(2),
            AnchorRef::new(ObjectId::generated(0), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::generated(1), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        ))
        .expect("endpoints are placed shapes");

    vec![
        ("box", render(&bare_box)),
        ("box-label", render(&box_label)),
        ("junction", render(&junction)),
        ("connector", render(&connector)),
        ("empty", ResolvedGrid::empty()),
    ]
}

#[test]
fn goldens_match_the_exported_bytes() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    for (name, grid) in scenarios() {
        let text = exporters::to_text(&grid);
        let svg = exporters::to_svg(&grid).unwrap_or_else(|err| panic!("{name}: {err}"));

        for (ext, bytes) in [("txt", text.as_str()), ("svg", svg.as_str())] {
            let path = Path::new(GOLDEN_DIR).join(format!("{name}.{ext}"));
            if update {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).expect("creates golden dir");
                }
                fs::write(&path, bytes).expect("writes golden file");
                continue;
            }
            let actual = fs::read_to_string(&path).unwrap_or_else(|_| {
                panic!(
                    "missing golden file {} — run with UPDATE_GOLDEN=1",
                    path.display()
                )
            });
            assert_eq!(
                actual, bytes,
                "{name}.{ext}: exported bytes differ from the golden file"
            );
        }
    }
}
