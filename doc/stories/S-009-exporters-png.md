---
title: S-009-exporters-png
description: PNG export via SVG rasterization — rasterizer decision (new ADR, resvg candidate), wasm-compatible, Err(Unsupported) path
scope: exporters
status: backlog
updated: 2026-10-22
---

# S-009: exporters — PNG rasterization

## Why

FR5 includes PNG; ADR 005 defers the rasterizer choice to a follow-up decision (candidate: `resvg`). The exporter must stay headless and testable (NFR5) and compile for wasm32 (NFR1); runtime offloading is S-011's job, not this crate's.

## Dependencies

- S-008 — rasterizes the SVG produced by `to_svg`

## Acceptance criteria

1. **AC1 — Rasterizer decision recorded.** New `doc/adr/014-<slug>.md` (from template, `status: proposed`) picks the rasterizer (verify `resvg` compiles for wasm32-unknown-unknown; record why alternatives were rejected); linked from ADR 005 + mkdocs nav.
2. **AC2 — API.** `to_png(&ResolvedGrid) -> Result<Vec<u8>, ExportError>` returns a valid PNG (signature bytes verified by test or decoder round-trip of dimensions). Check: unit test on a small grid; decoded width/height match the grid aspect.
3. **AC3 — Unsupported path.** The `ExportError::Unsupported` variant is constructible and covered by a test (path taken when no rasterizer backend is available at runtime/build). Check: unit test.
4. **AC4 — Deterministic bytes.** Same grid → byte-identical PNG across two calls. Check: unit test.
5. **AC5 — Wasm compatibility.** `cargo check -p exporters --target wasm32-unknown-unknown` exits 0 with the chosen backend.
6. **AC6 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p exporters` ≥ 80%.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
