---
title: S-010-ui-api-core-services
description: the ADR 009 seam — ui-api traits (Store/Renderer/Exporter/Catalog) pinned, core-services backends + Services::native()/web() composition
scope: ui-api/core-services
status: backlog
updated: 2026-10-22
---

# S-010: ui-api + core-services — service seam

## Why

[ADR 009](../adr/009-ui-api-service-layer.md) (proposed): the GUI talks to the core only through this trait seam; its follow-up requires pinning the trait signatures now so later backend swaps don't ripple into the GUI. Backends are thin wrappers over the context crates; platform IO stays in this layer (C3 rule).

## Dependencies

- S-004 — store backend wraps `ad-format` load/save
- S-005 — catalog backend wraps `shape-registry`
- S-006, S-007 — renderer backend wraps `render-pipeline`
- S-008, S-009 — exporter backend wraps `exporters`

## Acceptance criteria

1. **AC1 — Traits pinned in `ui-api`.** `DiagramStore { load, save }`, `Renderer { render }`, `Exporter { to_text, to_svg, to_png }`, `ShapeCatalog { list, get, anchors }` (signatures may be refined, but the four-service shape from ADR 009 is fixed) + error types (`StoreError`, `ExportError` re-exports); `ui-api` depends on no other workspace crate. Check: T5 audit passes; `cargo check -p ui-api`.
2. **AC2 — Backends.** `core-services` provides concrete backends wrapping the context crates (fs-backed store for native; in-memory where a platform has no FS) and a `Services { store, renderer, exporter, catalog }` with `native()` / `web()` constructors selecting per-target backends (cfg-gated). Check: unit tests exercising each backend against real core-crate behavior (tempdir round-trip through the store; render+export of a fixture model).
3. **AC3 — Seam testability.** A fake `Services` (in-memory implementations) exists in dev/test code and is usable by future GUI tests (T7 pattern established with one example test). Check: test in `core-services` using the fakes.
4. **AC4 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p ui-api -p core-services` ≥ 80% each.
5. **AC5 — ADR 009 flipped to accepted** (seam implemented as decided, signatures pinned); C3 concept doc Implementation section updated for the application layer; overview tracker updated.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
