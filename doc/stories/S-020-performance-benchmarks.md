---
title: S-020-performance-benchmarks
description: criterion benchmarks for render/export against proposed P1/P2 budgets + double-render determinism invariant (QS1)
scope: docs/tooling
status: done
updated: 2026-11-02
---

# S-020: performance benchmarks + determinism invariants

## Why

QS1/QS12 have proposed budgets awaiting confirmation (P1 first render < 100 ms for 500 objects; P2 export scales linearly). We can't gate on unconfirmed budgets, but we must *measure* them and pin the double-render determinism invariant so regressions are visible.

## Dependencies

- S-008 — render + export paths to benchmark
- S-011 — PNG timing through the worker path

## Acceptance criteria

1. **AC1 — Render benchmark.** Criterion benchmark: first render of a 500-object diagram (mix of shapes/text/connectors, generated deterministically) on `render-pipeline`; result recorded against the proposed P1 budget. Check: `cargo bench -p render-pipeline` runs; number recorded in Verification + noted in §6 (still "awaiting confirmation", not gated).
2. **AC2 — Export scaling.** Benchmark text/SVG/PNG export for N = 100/500/1000 objects; growth roughly linear (reported, not gated, per P2). Check: bench output recorded.
3. **AC3 — Determinism invariant test.** A workspace-level test renders a large fixture model twice (separate calls, fresh allocations) and asserts byte-identical text *and* SVG; also re-runs the S-004 round-trip property at higher iteration count as a scheduled check. Check: `cargo test` includes it; it fails when a mutation is injected (verify once, revert).
4. **AC4 — Gates green.** fmt/clippy/test pass; coverage unchanged ≥ 80%.

## Implementation notes

- **Fixtures** — `render_pipeline::bench_fixtures::mixed_diagram(n)` (public, in the lib so benches *and* tests share it): deterministic 50/100/500/1000-object diagrams from a fixed SplitMix64 seed (`0xA5C1_2026` — changing it invalidates baselines), cycling box/ellipse/class + free text + solid connectors between consecutive shapes, laid out on a 25×20 grid of 8×5 cells with positions clamped to the 200×100-cell P1/QS12 envelope (dense → overlapping shapes, which the pipeline handles). Unit-tested: within bounds + deterministic across calls.
- **Benches** — `crates/render-pipeline/benches/render_500.rs` (first render @ 50/500) and `crates/exporters/benches/export_scaling.rs` (text/SVG/PNG @ 100/500/1000, pre-resolved grid). Criterion 0.8 as a dev-dependency **target-gated to non-wasm** (`cfg(not(target_arch = "wasm32"))`) — criterion pulls rayon, which does not build for wasm; every bench item is cfg-gated individually and a stub `main` keeps the `[[bench]]` targets compiling under the wasip1 gate (crate-level `#![cfg]` strips the whole crate including any fallback main — verified in isolation).
- **Determinism invariant** — `crates/exporters/tests/determinism.rs`: 50-object fixture rendered + exported twice through separate calls with fresh allocations; text *and* SVG asserted byte-identical, plus a repeated-render stability check. 50 (not 500) objects keeps the suite inside tarpaulin's per-binary timeout in debug (500 took 26.6 s instrumented vs the 30 s limit); the P1 envelope scale is covered by the benches.
- **Scheduled heavy round-trip** — `ad-format::round_trip_heavy`: the S-004 lossless round-trip property re-run at 2560 cases (vs the default 256) as a scheduled check per AC3.

## Verification

- [x] **AC1** — `cargo bench -p render-pipeline` runs: first render of the deterministic 500-object fixture ≈ **409.6 µs median** (release, devcontainer x86_64) — ≈ 240× under the proposed P1 budget (< 100 ms); 50 objects ≈ 56.5 µs. Recorded in [§6](../arc42/06-quality-model.md) as a measured baseline; budget stays "proposed, pending confirmation", not gated.
- [x] **AC2** — `cargo bench -p exporters` runs (100/500/1000 objects): text 10.7 µs → 36.3 µs → 39.6 µs (sub-linear); SVG 38.1 ms → 291 ms → 410 ms (≈ linear, mildly super-linear over 10×); PNG 50 ms → 344 ms → 470 ms (≈ linear). Reported in §6; not gated per P2.
- [x] **AC3** — `cargo test` includes the determinism invariant (`exporters::determinism`, 2 tests) and the heavy round-trip (`ad-format::round_trip_heavy`, 2560 cases); all green in the 714-test workspace run. Mutation check performed once: a temporary wall-clock stamp injected into `to_text` made `two_independent_runs_are_byte_identical` fail (only the stamp line differed between runs), then the mutation was reverted and the test re-verified green — the invariant detects non-determinism.
- [x] **AC4** — gates green: `cargo fmt --all` clean; `cargo clippy --workspace --all-targets` (native) 0 warnings/errors; core-crate clippy on `wasm32-wasip1` (`--all-targets`) 0 — the new bench targets compile there via the cfg-gate + stub-`main` pattern; `cargo test --workspace` **714 passed, 0 failed**; tarpaulin (Llvm) workspace **93.01%** ≥ 80% (up from 92.95%); `node scripts/wasm-core-tests.mjs` all suites pass (the new fixture unit tests run on wasm too). Note: while running the wasip1 gate, S-019's AC5 wording was found to overstate its scope (`gui-app` does not compile for wasip1 — wgpu-hal gles E0277 under WASI p1); corrected in the S-019 story, no behavior change. 
