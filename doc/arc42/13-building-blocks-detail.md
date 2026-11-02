---
title: "arc42-13-building-blocks-detail"
description: detailed specification of the important building blocks identified in chapter 8
scope: architecture
status: stable
updated: 2026-11-02
---

# 13. Building Blocks (Detail)

> *Detailed specification of the important building blocks identified in [§8](08-building-block-view.md). One block per crate; interfaces are the actual public APIs as implemented.*

### BB1: `diagram-model` — Diagram aggregate (core bounded context)

- **ID:** BB1
- **Purpose:** the domain model — the `Diagram` aggregate root (C4) with identity-bearing objects (`ShapeInstance`, `ConnectorInstance`, text), value-object geometry (`CellPos`/`CellSize`, C5), anchors, tips, bends, colors, and shape-owned text regions; all invariants enforced at the boundary.
- **Properties / non-functional requirements:** no I/O, no GUI deps, no randomness — pure data + total operations (QS7 determinism); every fallible mutation returns `Result<_, ModelError>`; proptest property suite (`testgen`) pins round-trippable invariants.
- **Interfaces (provided/required):** provides the `Diagram` value type + `ModelError`; required by everything else (it sits at the bottom of the DAG). Depends on `std` only (+ `proptest` dev-dep).
- **Dependencies:** none (workspace root).
- **Implementation notes:** modules `anchors`, `color` (`Rgb`/`ColorRole`/`DiagramColors`), `geometry`, `glyphs` (svgbob-alphabet predicates shared with rendering), `invariants`, `objects`, `testgen`. Connector mutation API: `connect`, `add_bend`/`move_bend`/`remove_bend`, `set_tip` — each typed-error-checked (S-028/S-029).

### BB2: `ad-format` — `.ad` persistence bounded context

- **ID:** BB2
- **Purpose:** canonical lossless serialization of a `Diagram` (C6) — versioned, line-based, one statement per line with explicit cell coordinates; byte-stable on re-save (QS7).
- **Properties / non-functional requirements:** deterministic parser + serializer; positioned parse errors (`ParseError` with line/column); round-trip identity pinned by property tests at small *and* heavy scale (`round_trip_heavy`, 2560 cases, S-020); format version in the header (currently 1.1) with backward-compatibility tests for 1.0 files.
- **Interfaces (provided/required):** `parse`/`parse_document` → `Document`; `serialize`/`serialize_document` → canonical text; consumed by `core-services` (`FsStore`) and the GUI's file flows. Depends on BB1 only.
- **Dependencies:** BB1.
- **Implementation notes:** modules `error`, `parser`, `serializer`. Statement set covers shapes (+ per-region content), connectors (tips, bends via `via …`), colors, text; canonical ordering makes re-saves byte-identical.

### BB3: `shape-registry` — Shape Catalog bounded context

- **ID:** BB3
- **Purpose:** the shape catalog (C7/C8) — built-in templates (box, ellipse, UML set incl. the compartmented class with 3 text regions), user collections (`.adc` files), anchor sets, per-region declarations, and resize-rule re-rasterization (C10).
- **Properties / non-functional requirements:** deterministic lookup + rasterization (`rasterize()` is a pure function of template, rules, target size); positioned typed errors for all malformed collection input (`ParseError`, `AnchorSetError`, `RegionSetError`, `ResizeError`, `LookupError`); collections are plain text shareable via VCS.
- **Interfaces (provided/required):** `ShapeRegistry` (lookup/listing), `parse_collection`/`serialize_collection` (`.adc` I/O), `rasterize`/`min_size` (resize rules), geometry types (`CellPos`, `CellSize`, `Region`); consumed by BB4 (render) and via the seam by the GUI (catalog service). Depends on BB1.
- **Dependencies:** BB1.
- **Implementation notes:** modules `builtin`, `collections` loading in `core-services`, `geometry`, `registry`, `resize`, `serializer`, `templates`, `types`.

### BB4: `render-pipeline` — Rendering & Export context (resolve stage)

- **ID:** BB4
- **Purpose:** deterministic model → resolved character grid (C2, C9): place shapes (stamping templates, resolving text regions), route connectors (structural edges OR-merged with borders per C1; manual waypoint polylines; oblique segments as Bresenham `/`/`\` content), then resolve every occupied cell to exactly one glyph (C1 rule table).
- **Properties / non-functional requirements:** pure function `render(&Diagram, &ShapeRegistry) -> Result<ResolvedGrid, RenderError>` — no I/O, no clock, deterministic (QS7); measured ≈ 0.41 ms for a 500-object diagram at the P1 envelope ([§6](06-quality-model.md)).
- **Interfaces (provided/required):** `render`, `connector_routes` (seam helper), `ResolvedGrid`/`expand` (scale); consumed by BB5 and via the seam (`PipelineRenderer`). Depends on BB1 + BB3.
- **Dependencies:** BB1, BB3.
- **Implementation notes:** modules `connectors`, `edges`, `ellipse`, `grid`, `glyphs` re-export; plus `bench_fixtures` — deterministic SplitMix64-seeded fixtures (fixed seed, 25×20 grid clamped to the 200×100-cell P1/QS12 envelope) shared by the criterion benches and the S-020 determinism test.

### BB5: `exporters` — Rendering & Export context (output stage)

- **ID:** BB5
- **Purpose:** resolved grid → outputs (C2): text (grid as-is), SVG (svgbob interpretation, ADR 005), PNG (rasterizer per ADR 016; optional backend — degrades to `Unsupported` when absent).
- **Properties / non-functional requirements:** byte-stable for identical input (QS7) — pinned by the determinism invariant test (`tests/determinism.rs`: two independent render+export passes ⇒ identical text *and* SVG); colors flow in via `DiagramColors` without touching text output (NFR2).
- **Interfaces (provided/required):** `to_text`, `to_svg(_with_options)` + `SvgOptions`, `to_png(_with_options)` + `RgbaImage`; consumed via the seam (`GridExporter`). Depends on BB4's grid type (via BB1/BB3 value types) + svgbob + optional raster backend.
- **Dependencies:** BB1, BB4 (grid), external: `svgbob`, PNG rasterizer (ADR 016).
- **Implementation notes:** modules `text`, `svg`, `png`; benches in `benches/export_scaling.rs` (text/SVG/PNG @ 100/500/1000 objects, criterion, native-only).

### BB6: `ascii-import` — Import bounded context (v1+, stub)

- **ID:** BB6
- **Purpose:** anti-corruption layer for foreign ASCII art (FR12) — segmentation + template matching against the Shape Catalog; unknown regions quarantined as reduced objects with warnings. **Out of v1 scope.**
- **Properties / non-functional requirements:** none yet — scaffolded stub so the §8.3 dependency DAG is complete and machine-checkable.
- **Interfaces (provided/required):** none public yet; will expose an import service over BB1 + BB3.
- **Dependencies:** (planned) BB1, BB3.
- **Implementation notes:** `lib.rs` doc-comment only; no logic.

### BB7: `ui-api` — application-layer seam (traits)

- **ID:** BB7
- **Purpose:** the service seam (ADR 009): trait definitions + shared types so the presentation layer never depends on a concrete backend — GUI code is testable against fakes, and backends are swappable.
- **Properties / non-functional requirements:** dependency target of `gui-app` *and* `core-services`; no behavior of its own (traits + types only); re-exports core error/grid types so app code sees one vocabulary.
- **Interfaces (provided/required):** traits `DiagramStore`, `Renderer`, `Exporter`, `ShapeCatalog`; seam error types (`CatalogError`, `ExportError`, `StoreError`); shared value types (template info, document handles). Depends on BB1–BB4 types only.
- **Dependencies:** BB1–BB4 (type-level).
- **Implementation notes:** modules `error`, `types`.

### BB8: `core-services` — application-layer backends + composition root

- **ID:** BB8
- **Purpose:** the concrete implementations of the seam traits + the composition root wiring them into a `Services` bundle; also hosts cross-cutting app services (PNG worker, preview hashing, collection loading).
- **Properties / non-functional requirements:** target-specific behavior is cfg-gated *here and in gui-app only* (Q2.2) — filesystem store native / in-memory web, PNG worker thread native / inline web (`webio`); `fakes` module provides in-memory backends for headless GUI tests; the T5 audit test machine-checks the §8.3 crate dependency DAG.
- **Interfaces (provided/required):** `Services::native()` / `native_with_collections(dir)` / web variant; `RegistryCatalog`, `PipelineRenderer`, `GridExporter`, `FsStore`/web store, `PngWorker`, `load_from_dir`/`load_from_text` (+ warnings); consumed by `gui-app`.
- **Dependencies:** BB1–BB5, BB7.
- **Implementation notes:** modules `catalog`, `collections`, `exporter`, `fakes`, `preview`, `registry_state` (shared registry state), `renderer`, `services`, `store`, `webio`, `worker`.

### BB9: `gui-app` — presentation layer (egui/eframe)

- **ID:** BB9
- **Purpose:** the application UI (ADR 008): canvas (WYSIWYG paint + overlays), sidebar (builtin categories incl. UML subcategories, collections with pre-rendered previews, filter), menus/dialogs (options, colors, template editor, file dialogs), Material app bar with window controls, status bar; one lib (`rlib` for native/tests) + thin entry points (`main.rs` native, `web.rs` Trunk wasm entry as `cdylib`).
- **Properties / non-functional requirements:** immediate-mode egui 0.37 — no retained widget tree to invalidate; all logic delegated through the seam (BB7/BB8); headless-testable via the tier-0b frame-pumping harness (`gui_tests`) and golden-image visual tests; feature bar: every feature story ships a UI-driven diagram decomposition test.
- **Interfaces (provided/required):** provides the executable application (native binary + web bundle); requires BB7 traits + BB8 `Services` (constructed per target). Platform cfg-gates: rfd file dialogs native / in-memory store + download web; PNG worker thread native / inline web; undecorated window + client-side resize gesture native / browser chrome web.
- **Dependencies:** BB7, BB8 (transitively all core crates), external: `eframe`/`egui`, `rfd` (native), Trunk build tooling (web).
- **Implementation notes:** modules `app` (state + event handling), `appbar`, `canvas`, `config` (`AppConfig` persistence), `dialogs`, `editor` (template editor), `exportio`, `fileio`, `gui_tests` (harness), `screenshot` (software rasterizer for goldens), `sidebar`, `template_editor`, `theme` (Material palettes), `web`/`webio`.
