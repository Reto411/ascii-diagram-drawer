---
title: "009 - ui-api-service-layer"
description: GUI talks to core only through a small trait-based service layer (ui-api) with pluggable backends wired at a composition root — loose coupling, per-target stores, swappable PNG rasterizer, fakeable GUI tests
scope: architecture
status: proposed
version: 1
updated: 2026-10-18
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
- **`core-services`** — concrete backends + composition root. Contains the trait implementations and a `Services { store, renderer, exporter, catalog }` aggregate with per-target constructors (`native()` / `web()`, PNG optional).

The four services:

| Service | Trait responsibility | Backends |
|---------|----------------------|----------|
| `DiagramStore` | load/save `.ad` files (FR4) | native fs store; browser store (wasm) |
| `Renderer` | `render(&Diagram, &ShapeCatalog) -> ResolvedGrid` | render-pipeline (thin wrapper over its pure functions) |
| `Exporter` | grid → text / SVG / PNG (FR2/FR5); `png()` may return `Err(Unsupported)` | svgbob exporter + optional rasterizer backend |
| `ShapeCatalog` | list/lookup shape types & anchor sets; registration (v1+, FR12) | builtin catalog + file-backed collections |

Rules:

- **`gui-app` depends only on `ui-api`, `core-services` (construction only), and `diagram-model` value types** — never on `ad-format`, `shape-registry`, `render-pipeline`, or `exporters` directly. All behavior goes through the traits (`Arc<dyn …>` held by the eframe app struct; dispatch cost is negligible at our scale).
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
- **Negative / costs:** one seam crate + one services crate; risk of a poorly-shaped API surface — mitigated by keeping exactly four traits derived from actual GUI needs and reviewing them at scaffolding time; `dyn` dispatch (negligible here).
- **Follow-ups:** pin exact trait signatures when the workspace is scaffolded; decide where platform store impls live (`core-services` vs a `platform-io` crate); revisit if egui's testing story matures.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v1 (2026-10-18):** initial decision — UI API seam with four services (`DiagramStore`, `Renderer`, `Exporter`, `ShapeCatalog`), `core-services` composition root, direct model mutation kept unabstracted.
