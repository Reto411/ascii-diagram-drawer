//! S-020 (AC1): render-scaling benchmarks — `cargo bench -p render-pipeline`.
//!
//! Measures first-render time for the 50-object and 500-object deterministic
//! fixtures from [`render_pipeline::bench_fixtures`], against the §6 P1 budget
//! ("first render < 100 ms" at ≤ 500 objects). Benches are native-only:
//! criterion's harness is not available on wasm targets, so every item is
//! cfg-gated and a stub `main` keeps the bench target compiling there
//! (the workspace gate includes `--all-targets --target wasm32-wasip1`).

#[cfg(not(target_arch = "wasm32"))]
use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(not(target_arch = "wasm32"))]
use render_pipeline::bench_fixtures::mixed_diagram;
#[cfg(not(target_arch = "wasm32"))]
use shape_registry::ShapeRegistry;

/// AC1: first-render time at 50 objects (representative interactive size).
#[cfg(not(target_arch = "wasm32"))]
fn render_50(c: &mut Criterion) {
    let diagram = mixed_diagram(50);
    let registry = ShapeRegistry::builtin();
    c.bench_function("render_first/50_objects", |b| {
        b.iter(|| render_pipeline::render(&diagram, &registry));
    });
}

/// AC1: first-render time at 500 objects — must stay under the §6 P1 budget
/// (< 100 ms); the criterion baseline records the measured value.
#[cfg(not(target_arch = "wasm32"))]
fn render_500(c: &mut Criterion) {
    let diagram = mixed_diagram(500);
    let registry = ShapeRegistry::builtin();
    c.bench_function("render_first/500_objects", |b| {
        b.iter(|| render_pipeline::render(&diagram, &registry));
    });
}

#[cfg(not(target_arch = "wasm32"))]
criterion_group!(benches, render_50, render_500);
#[cfg(not(target_arch = "wasm32"))]
criterion_main!(benches);

/// Keeps the bench target compiling on wasm (no criterion there).
#[cfg(target_arch = "wasm32")]
fn main() {}
