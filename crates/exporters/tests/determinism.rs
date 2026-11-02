//! QS1/QS12 (S-020 AC3): determinism is a CI invariant, not a one-time check.
//!
//! A large fixture diagram (50 objects — the representative interactive size,
//! cycling all built-in shape kinds) is rendered and exported twice through
//! **separate calls with fresh allocations** — two independent generator passes,
//! two independent renders — and both the text and the SVG must be byte-identical.
//! Any platform-dependent behavior in rendering or export (hash ordering,
//! allocator address leakage, time dependence) fails this test. The 500-object
//! P1 envelope is covered by the benchmarks instead: a second full pass at that
//! scale would push this suite past tarpaulin's per-binary timeout in debug.

use diagram_model::Diagram;
use exporters::{to_svg, to_text};
use render_pipeline::bench_fixtures::mixed_diagram;
use render_pipeline::render;
use shape_registry::ShapeRegistry;

/// One full pass: fresh fixture → fresh render → text + SVG.
fn export_pass(diagram: Diagram) -> (String, String) {
    let grid = render(&diagram, &ShapeRegistry::builtin()).expect("fixture renders");
    let text = to_text(&grid);
    let svg = to_svg(&grid).expect("svg export");
    (text, svg)
}

/// Two independent runs must be byte-identical — text and SVG (QS1: 0 diff bytes).
#[test]
fn two_independent_runs_are_byte_identical() {
    let (text_a, svg_a) = export_pass(mixed_diagram(50));
    let (text_b, svg_b) = export_pass(mixed_diagram(50));
    assert_eq!(
        text_a, text_b,
        "text must be byte-identical across independent runs"
    );
    assert_eq!(
        svg_a, svg_b,
        "SVG must be byte-identical across independent runs"
    );
}

/// Repeated renders of one diagram instance are stable (no hidden state in the
/// pipeline or the registry).
#[test]
fn repeated_render_of_one_diagram_is_stable() {
    let diagram = mixed_diagram(50);
    let registry = ShapeRegistry::builtin();
    let grid_a = render(&diagram, &registry).expect("renders");
    let grid_b = render(&diagram, &registry).expect("renders");
    assert_eq!(to_text(&grid_a), to_text(&grid_b));
    assert_eq!(to_svg(&grid_a).expect("svg"), to_svg(&grid_b).expect("svg"));
}
