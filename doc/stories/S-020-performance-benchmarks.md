---
title: S-020-performance-benchmarks
description: criterion benchmarks for render/export against proposed P1/P2 budgets + double-render determinism invariant (QS1)
scope: docs/tooling
status: backlog
updated: 2026-10-22
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

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
