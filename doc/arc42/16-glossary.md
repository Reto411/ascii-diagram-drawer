---
title: "arc42-16-glossary"
description: project-specific terms — one shared vocabulary for docs and code
scope: architecture
status: stable
updated: 2026-11-02
---

# 16. Glossary

> *Definitions of project-specific terms, so the documentation (and code) share one vocabulary. Concept terms link to their concept doc in [§12](12-concepts/index.md).*

| Term | Definition |
|------|------------|
| **Glyph resolution** | Assigning exactly one svgbob input-alphabet character to each occupied grid cell, based on all edges/content passing through it (concept C1 — [glyph-resolution](12-concepts/glyph-resolution.md), ADR 001) |
| **Cell grid** | The unbounded 2D integer grid a diagram is placed on and rendered into; top-left origin, whole-diagram scale, one cell = one character at scale 1 (concept C5 — [cell-grid](12-concepts/cell-grid.md)) |
| **Diagram object** | A placeable element on the canvas — a predefined or user-defined shape, connector, or text; user-defined objects are stored in collections ([§1.1](01-introduction-and-context.md#11-objectives) Obj. 3, [ADR 007](../adr/007-user-defined-diagram-objects.md)) |
| **Diagram aggregate** | The core domain model — `Diagram` root with identity-bearing objects, value-object geometry, invariants enforced at the boundary; single source of truth for everything rendered/exported (concept C4 — [diagram-aggregate](12-concepts/diagram-aggregate.md)) |
| **Shape template** | A reusable shape definition — named grid of svgbob-alphabet characters with declared anchor cells, optional text regions and resize rules; predefined (built-in) or user-defined in collections; stamped onto the cell grid at render time (concept C7 — [shape-template](12-concepts/shape-template.md), [ADR 007](../adr/007-user-defined-diagram-objects.md)) |
| **Shape collection** | A named plain-text file (`.adc`) grouping shape templates — the shareable unit of user content; identity = filename stem, `collection/name` reference form (concept C8 — [collection](12-concepts/collection.md)) |
| **Text region** | A named cell rectangle inside a shape template that carries per-instance text content (e.g. the name/attributes/methods compartments of the UML class); declared in `.adc`, filled per instance in `.ad` (FR19, CR-016; [shape-template](12-concepts/shape-template.md)) |
| **Resize rules** | Per-edge grow/shrink character patterns (+ even/odd parity variants) a template declares so its instances can be resized; re-rasterization is a pure function of (template, rules, target size) (concept C10 — [resize-rules](12-concepts/resize-rules.md)) |
| **Render pipeline** | The deterministic path model → resolved character grid → text/SVG/PNG; vector output is derived from the same grid as the text (concept C2 — [render-pipeline](12-concepts/render-pipeline.md)) |
| **Resolved grid** | The intermediate character grid after all shapes, connectors and text are placed and every occupied cell resolved to exactly one glyph; never stored as source data — derived on every render/export (C2) |
| **Structural edge** | An axis-aligned connector segment rendered as a box-drawing line that participates in glyph resolution (OR-merges with borders at junctions per C1 F2); oblique segments are instead Bresenham `/`/`\` content (C9 — [connector-routing](12-concepts/connector-routing.md)) |
| **Connector routing** | How connectors become routed lines in the resolved grid — endpoints anchor-bound or free, automatic straight/single-corner L, manual waypoint bends, selectable per-end tips; re-resolved on every render (concept C9 — [connector-routing](12-concepts/connector-routing.md), ADR 013) |
| **Tip** | The optional arrowhead/rhombus glyph stamped at a connector end (`Bare`, `HollowArrow`, `EmptyRhombus ◇`, `FilledRhombus ◆`), selectable per end (CR-008, ADR 013 v5) |
| **Waypoint / bend** | A user-placed cell on a connector's route — the polyline is `[end₁, bends…, end₂]`; dragging a connector body adds one at the drop point; collinear drops are removed (Visio-style) (CR-007, ADR 013 v4) |
| **Bounded context** | The strategic DDD decomposition — five bounded contexts (+ application/presentation layer), each owning one model/language, realized as crate boundaries (concept C3 — [bounded-contexts](12-concepts/bounded-contexts.md), ADR 011) |
| **Service seam** | The application-layer boundary the GUI codes against — `ui-api` traits (`DiagramStore`, `Renderer`, `Exporter`, `ShapeCatalog`) implemented by `core-services` backends or fakes (ADR 009) |
| **WYSIWYG canvas** | The canvas paints exactly what export produces: fixed glyph resolution and background, with selection/anchor/resize overlays drawn on top of — never into — the rendered surface; pinned by golden-file tests |
| **`.ad` file** | The canonical lossless diagram format — versioned (currently 1.1), line-based, one statement per line with explicit cell coordinates; byte-stable on re-save (concept C6 — [diagram-file-format](12-concepts/diagram-file-format.md), ADR 004) |
| **Feature bar** | The usability gate: every feature story ships at least one `ui_decomposition_*` test drawing a representative diagram purely through the real UI in the headless harness (AGENTS.md conventions, gui-testing skill) |
