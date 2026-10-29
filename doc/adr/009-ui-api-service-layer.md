---
title: "009 - ui-api-service-layer"
description: GUI talks to core only through a small trait-based service layer (ui-api) with pluggable backends wired at a composition root — loose coupling, per-target stores, swappable PNG rasterizer, fakeable GUI tests; five services incl. ASCII import (v1+)
scope: architecture
status: accepted
version: 7
updated: 2026-10-29
---

# ADR 009: UI API service layer with pluggable backends

## Context

[§8 Building Block View](../arc42/08-building-block-view.md) originally wired `gui-app` directly against all five core crates. Three recorded problems make that seam too tight:

1. **PNG rasterizer is undecided** (risk R3, [§2](../arc42/02-architecture-vision.md)) — a direct dependency would hardwire the GUI to an unmade choice; v1's possible degradation to SVG-only must not require GUI changes.
2. **Per-target file IO** ([ADR 008](008-gui-framework-choice.md) consequences) — native filesystem vs browser file APIs are different backends behind the same "open/save" gesture.
3. **GUI testability** (T7, [§6.3](../arc42/06-quality-model.md)) — egui's testing tooling is limited; GUI logic should be runnable against in-memory fakes without touching real rendering or disk.

A future headless/CLI export path would also duplicate the wiring if services are not centralized. The requested direction: a **UI API** layer that delegates to corresponding backends.

## Decision

Introduce two crates forming the seam between GUI and core:

- **`ui-api`** — the service interface layer. Defines four small, object-safe traits plus shared service types (e.g. `StoreError`). Depends only on `diagram-model` and `render-pipeline` (for the `ResolvedGrid` type).
- **`core-services`** — concrete backends + composition root. Contains the trait implementations and a `Services { store, renderer, exporter, catalog, png_worker }` aggregate with per-target constructors (`native()` / `web()`, PNG optional). The aggregate also exposes the [ADR 012](012-responsive-ui-threading.md) PNG worker (`png_worker: Arc<PngWorker>`) so the GUI's non-blocking export flow reaches it without a new dependency edge (v7); and `ui-api` re-exports the render-pipeline types that already appear in its trait signatures (`ResolvedGrid`, `RenderError`) so consumers can name them within the documented edge (v7).

Five services (the last is v1+ scope):

| Service | Trait responsibility | Backends |
|---------|----------------------|----------|
| `DiagramStore` | load/save `.ad` files (FR4) — `load` is the importer for *our own* format: file bytes → `ad-format` parser → `LoadedDocument { diagram, skipped_unknowns }` (v4) | native fs store; browser store (wasm) |
| `Renderer` | `render(&Diagram) -> ResolvedGrid` — the shape catalog is **bound at construction** (the GUI never passes registry types across the seam) | render-pipeline (thin wrapper over its pure functions) |
| `Exporter` | grid → text / SVG / PNG (FR2/FR5); `to_png()` may return `Err(Unsupported)` without a rasterizer backend ([ADR 016](016-png-rasterizer.md)) | svgbob exporter + optional rasterizer backend |
| `ShapeCatalog` | list/lookup shape types & anchor sets; register user templates (S-017, FR11) | shared registry: builtin catalog + file-backed collections (`registry_state.rs`) |
| `DiagramImporter` | **ASCII import of foreign diagrams** (F9, **v1+**): text → grid → region segmentation → template matching → editable `Diagram`; unknown regions → fixed-size reduced objects + warnings (FR12) | new `ascii-import` core crate (pure logic); adapter in `core-services` |

Rules:

- **`gui-app` depends only on `ui-api`, `core-services` (construction only), and `diagram-model` value types** — never on `ad-format`, `shape-registry`, `render-pipeline`, `exporters`, or `ascii-import` directly. All behavior goes through the traits (`Arc<dyn …>` held by the eframe app struct; dispatch cost is negligible at our scale).
- `ascii-import` sits in the **core tier** (depends on model, registry, pipeline grid type) — pure and headless-testable like the rest of core; only reachable through the seam.
- **Deliberate non-abstraction:** diagram *mutation* (place/move/resize/connect) stays direct manipulation of the pure `Diagram` value type inside the GUI — no "model service" trait. The data is cheap to clone and involves no IO; abstracting it would be ceremony without a backend to swap.
- Graceful degradation by construction: if no PNG backend is configured, `Exporter::png()` returns `Err(Unsupported)` and the GUI offers SVG-only export — no code change (R3 resolved as an option, not a fork).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Direct `gui-app → core` dependencies (original §8) | Hardwires PNG choice, per-target IO, and GUI testing to concrete implementations; headless path would re-wire everything |
| Traits defined inside each core crate | GUI still depends on every concrete crate's API; no single seam; fakes must pull in real crates |
| Full DI framework / service container | Overkill for four services in Rust; a manual composition root is idiomatic and reviewable |

## Consequences

- **Positive:** backends swappable per target (store) and per build configuration (PNG); GUI logic testable with in-memory fakes (T7); headless/CLI export reuses the same `Services`; the open PNG decision (R3) stops blocking architecture.
- **Relation to [ADR 011](011-domain-driven-design.md):** the seam is reinterpreted as the *application-layer* boundary of the DDD structure — this decision is unchanged.
- **Negative / costs:** one seam crate + one services crate; risk of a poorly-shaped API surface — mitigated by keeping exactly four traits derived from actual GUI needs and reviewing them at scaffolding time; `dyn` dispatch (negligible here).
- **Follow-ups:** ~~pin exact trait signatures~~ (done in S-010, v3; `load` widened to `LoadedDocument` in v4); decide where platform store impls live (`core-services` vs a `platform-io` crate) — S-010 put both stores in `core-services`, revisit if wasm needs more; ASCII-import matching heuristics (segmentation + tolerance scoring) are an open spec point for v1+ — pin with golden tests when implemented; revisit if egui's testing story matures.
- **Auxiliary text output (S-014):** the `.txt` sink of "Copy/Save text" is *not* a `.ad` document, so it does not go through `DiagramStore`; `gui-app` computes the exact bytes via `Exporter::to_text` and writes them with one small cfg-gated platform helper (native: `std::fs`; wasm: typed error until the browser download lands in S-019). This is the single sanctioned direct-file-access exception in the view layer — `.ad` IO still goes exclusively through the seam.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v7 (2026-10-29):** two wiring additions for S-015 (SVG/PNG export + scale), no behavior change: (1) `ui-api` now re-exports `render_pipeline::{ResolvedGrid, RenderError}` — they already appear in the `Renderer`/`Exporter` signatures but were unnameable by consumers, and `gui-app` may not link `render-pipeline` directly (dependency rule above); (2) the composition root gains `Services.png_worker: Arc<PngWorker>` (target-aware via new `PngWorker::for_target()` — threaded on native, inline on wasm32, per ADR 012's "one API either way"), so the GUI's export flow submits/polls through the seam crate without touching worker internals. The worker stays a mechanism of [ADR 012](012-responsive-ui-threading.md), not a fifth service trait — it has no fakeable contract beyond what `PngJob::poll` already offers, and tests drive it with real or channel-wrapped jobs.
- **v6 (2026-10-29):** the seam gains its first write path — `ShapeCatalog::register(collection, NewTemplate) -> Result<(), CatalogError>` and `Renderer::preview_pattern(&[String])` (S-017, FR3/FR11). Why: the template editor must persist user templates into collection `.adc` files and preview them through the *real* pipeline before saving; both are service capabilities, so they cross the seam instead of touching registry types or file IO in `gui-app`. `NewTemplate { name, rows, anchors }` is plain seam-safe data (ragged rows — the backend right-pads per the parser rule); `CatalogError::{Io, Invalid}` mirrors `StoreError`'s shape. Backends: `core-services registry_state.rs` holds one `ShapeRegistry` shared by renderer and catalog (the only interior mutability in the crate) — registration updates both at once, so the sidebar lists a saved template and the renderer resolves it with no re-composition; persistence is canonical `.adc` rewrite via new `shape-registry serializer.rs` (`parse(serialize(c)) == c`, golden-pinned). `Renderer::preview_pattern` places the pattern in an otherwise-empty diagram through the same place/resolve stages, so preview bytes equal saved-template render bytes (unit-tested). Registration on a built-in-only composition (e.g. `web()` before S-019) fails with `CatalogError::Io` — no collections directory configured.
- **v5 (2026-10-29):** the catalog read model grows two fields — `ShapeInfo.collection: Option<String>` (the collection a template belongs to; `None` = built-in) and `ShapeInfo.preview: Vec<String>` (the glyph pattern rows, so the sidebar can show a small preview without touching registry types) — and the composition root gains `Services::native_with_collections(dir) -> (Services, Vec<CollectionLoadWarning>)`, which discovers `*.adc` files in a configured directory at startup (S-016 AC2). Why: FR3/FR6 need the GUI to group the sidebar by collection and preview patterns; discovery is an *application* concern per the [collection concept](../arc42/12-concepts/collection.md) (the search path is not part of the format spec), so it lives in `core-services` (`collections.rs`), never in `shape-registry`. Invalid files become warnings with file + reason — startup always succeeds (RS family). The web mechanism (user-supplied collection text) lands with the wasm entry point in S-019; until then the wasm build uses the built-in catalog only.
- **v4 (2026-10-29):** `DiagramStore::load` now returns `LoadedDocument { diagram, skipped_unknowns }` instead of a bare `Diagram`. Why: RS2 (parse errors shown with line/column, *nothing silently dropped*) + S-014 AC4 (no silent data loss) require the GUI to know when a load dropped forward-compat statements — the parser preserves unknown statements of newer minor versions in its `Document`, but the bare-`Diagram` seam return discarded that knowledge (and with it any chance to warn before an overwrite). The count is the minimal read model: carrying `ad-format::Document` across the seam would force `ui-api` to depend on `ad-format`, breaking this ADR's dependency rule. Consequence: preservation-on-resave of unknown statements is *not* possible at the seam today (the GUI saves the edited `Diagram` via `serialize`, which cannot re-emit dropped lines) — so per S-014 AC4 the GUI warns before overwriting a document that came from a file with skipped statements, instead of silently losing them. Full preservation (stable slot identity across edits + `Document` across the seam) is a follow-up if/when newer minor versions actually ship.
- **v3 (2026-10-28):** signatures pinned and implemented in S-010 — `Renderer::render(&self, &Diagram)` binds the shape catalog at construction instead of taking it per call (the GUI never passes registry types across the seam); exporter methods are `to_text`/`to_svg`/`to_png`; seam error types are `StoreError::{Io, Parse{line,column,message}}` and `ExportError::{Vector, Raster, Unsupported}`; read models `ShapeInfo`/`AnchorInfo` carry geometry as `diagram-model` value objects. Status flipped to accepted.
- **v2 (2026-10-18):** added `DiagramImporter` as the fifth service (v1+, F9/FR12) — ASCII import of foreign diagrams goes through the same seam instead of being special-cased in `gui-app` or buried in `shape-registry`; the pure matching logic gets its own `ascii-import` core crate.
- **v1 (2026-10-18):** initial decision — UI API seam with four services (`DiagramStore`, `Renderer`, `Exporter`, `ShapeCatalog`), `core-services` composition root, direct model mutation kept unabstracted.
