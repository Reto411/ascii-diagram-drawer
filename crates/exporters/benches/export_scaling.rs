//! S-020 (AC1): export-scaling benchmarks — `cargo bench -p exporters`.
//!
//! Measures text / SVG / PNG export time for the 100/500/1000-object
//! deterministic fixtures from [`render_pipeline::bench_fixtures`] against a
//! pre-resolved grid (AC2: growth roughly linear — reported, not gated, per P2).
//! Benches are native-only: criterion's harness is not
//! available on wasm targets, so every item is cfg-gated and a stub `main`
//! keeps the bench target compiling there (the workspace gate includes
//! `--all-targets --target wasm32-wasip1`).

#[cfg(not(target_arch = "wasm32"))]
use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(not(target_arch = "wasm32"))]
use exporters::{SvgOptions, to_png_with_options, to_svg, to_text};
#[cfg(not(target_arch = "wasm32"))]
use render_pipeline::bench_fixtures::mixed_diagram;
#[cfg(not(target_arch = "wasm32"))]
use shape_registry::ShapeRegistry;

/// One export trio (text + SVG + PNG) for a diagram of `objects` objects.
#[cfg(not(target_arch = "wasm32"))]
fn export_bench(c: &mut Criterion, objects: u32) {
    let diagram = mixed_diagram(objects);
    let grid =
        render_pipeline::render(&diagram, &ShapeRegistry::builtin()).expect("fixture renders");
    let text_id = format!("export_text/{objects}_objects");
    c.bench_function(text_id.as_str(), |b| {
        b.iter(|| to_text(&grid));
    });
    let svg_id = format!("export_svg/{objects}_objects");
    c.bench_function(svg_id.as_str(), |b| {
        b.iter(|| to_svg(&grid));
    });
    let png_id = format!("export_png/{objects}_objects");
    let opts = SvgOptions::default();
    c.bench_function(png_id.as_str(), |b| {
        b.iter(|| to_png_with_options(&grid, &opts));
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn export_100(c: &mut Criterion) {
    export_bench(c, 100);
}

#[cfg(not(target_arch = "wasm32"))]
fn export_500(c: &mut Criterion) {
    export_bench(c, 500);
}

#[cfg(not(target_arch = "wasm32"))]
fn export_1000(c: &mut Criterion) {
    export_bench(c, 1000);
}

#[cfg(not(target_arch = "wasm32"))]
criterion_group!(benches, export_100, export_500, export_1000);
#[cfg(not(target_arch = "wasm32"))]
criterion_main!(benches);

/// Keeps the bench target compiling on wasm (no criterion there).
#[cfg(target_arch = "wasm32")]
fn main() {}
