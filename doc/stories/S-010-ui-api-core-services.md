---
title: S-010-ui-api-core-services
description: the ADR 009 seam — ui-api traits (Store/Renderer/Exporter/Catalog) pinned, core-services backends + Services::native()/web() composition
scope: ui-api/core-services
status: done
updated: 2026-10-28
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

- `ui-api` (traits pinned, ADR 009 v3): `DiagramStore { load, save }`, `Renderer { render }`, `Exporter { to_text, to_svg, to_png }`, `ShapeCatalog { list, get, anchors }`; seam error types `StoreError::{Io, Parse{line,column,message}}` and `ExportError::{Vector, Raster, Unsupported}`; read models `ShapeInfo`/`AnchorInfo` carry geometry as `diagram-model` value objects. Depends on `diagram-model` + `render-pipeline` for the `Diagram`/`ResolvedGrid` value types — exactly as ADR 009 allows (AC1's "no other workspace crate" wording was stricter than the ADR; the ADR wins).
- `core-services`: one module per backend — `store` (`FsStore` over real files, `MemoryStore` keyed by virtual path), `renderer` (`PipelineRenderer`, registry bound at construction), `exporter` (`GridExporter`, maps every `exporters::ExportError` variant to the seam), `catalog` (`RegistryCatalog`, builtins + collections as read models). Both stores live in `core-services` (ADR 009 follow-up decided; no separate `platform-io` crate for now).
- `Services { store, renderer, exporter, catalog }` with `native()`/`web()` — **not** cfg-gated: both constructors compile on every target and differ only in the store backend, so a wasm build calling `native()` fails loudly at construction rather than silently misbehaving; the GUI picks per target.
- `fakes.rs` (public): `FakeStore` (seedable), `FakeRenderer` (fixed grid or typed error), `FakeExporter` (fixed payloads for all three media), `FakeCatalog` (explicit read models) — the T7 pattern, exercised by one example test through `Arc<dyn …>` exactly as the GUI will hold them.
- Geometry crosses from `shape-registry`'s local value objects into `diagram-model`'s shared kernel at the seam (`CellPos{x,y}` / `CellSize{width,height}` field-for-field) — the C3 rule in action.

## Verification

- [x] AC1 — four traits + error types pinned in `ui-api`; `cargo check -p ui-api` clean; T5 dependency audit passes (`every_internal_dependency_edge_is_on_the_allow_list` green with the new edges).
- [x] AC2 — backends tested against real core behavior: `fs_store_round_trips_a_real_file` (tempdir save→parse→equal diagram, canonical header), parse errors carry line/column `(2, 11)`, missing file → typed `Io`; `renders_exactly_like_the_pipeline` + typed `UnknownShapeType`; `exports_all_three_media` (text/SVG/PNG signature on a real grid); catalog tests cover builtins (9×5 box, n/s/w/e anchors), qualified collection ids, unknown-id `None`s.
- [x] AC3 — `fakes.rs` + two example tests: `fakes_drive_the_same_trait_objects` (full load→render→export→catalog flow through trait objects, incl. a fakeable error path) and `fakes_answer_through_their_traits` (every fake's full surface).
- [x] AC4 — fmt/clippy (0 warnings)/test green workspace-wide; tarpaulin: ui-api 16/16 instrumented lines (100%), core-services 114/125 (91%); workspace total 97.36% ≥ 80%.
- [x] AC5 — ADR 009 → accepted v3 (signatures pinned, Renderer binds catalog at construction); C3 concept doc Implementation section updated; overview tracker updated. 
