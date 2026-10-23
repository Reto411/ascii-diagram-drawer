---
title: "arc42-12-concepts"
description: index of architecture concepts (C1+) — each concept has a full doc in doc/arc42/12-concepts/
scope: architecture
status: draft
updated: 2026-10-23
---

# 12. Concepts

> *Important concepts with stable identities — the vocabulary of the architecture (e.g. "DiagramModel", "Cell grid", "Box-drawing renderer").*
>
> *This chapter is the **index**. Full concept docs live in this folder (`doc/arc42/12-concepts/`, one file per concept, created from `doc/templates/concept.md`).*

## Concept C1: Glyph Resolution

- **ID:** C1
- **Purpose:** the renderer assigns one matching svgbob-alphabet character per occupied grid cell, wherever possible (corners, T-junctions, crossings)
- **Details:** [`doc/arc42/12-concepts/glyph-resolution.md`](glyph-resolution.md) · decided in [ADR 001](../../adr/001-glyph-resolution-alphabet.md) (v1: mechanism, v2: svgbob input alphabet)
- **Interfaces (provided/required):** [TBD]
- **Dependencies:** [TBD]

## Concept C2: Render Pipeline

- **ID:** C2
- **Purpose:** deterministic model → resolved character grid → text/SVG/PNG; vector output is derived from the same grid as the text (svgbob-style interpretation), so all outputs match exactly
- **Details:** [`doc/arc42/12-concepts/render-pipeline.md`](render-pipeline.md) · rendering reference: svgbob (§1 Background)
- **Interfaces (provided/required):** [TBD]
- **Dependencies:** C1 (glyph resolution is stage 2 of the pipeline)

## Concept C3: Bounded Contexts

- **ID:** C3
- **Purpose:** the strategic DDD decomposition — five bounded contexts (+ application/presentation layer), each owning one model/language, realized as crate boundaries in one binary; context mapping defines how they talk
- **Details:** [`doc/arc42/12-concepts/bounded-contexts.md`](bounded-contexts.md) · decided in [ADR 011](../../adr/011-domain-driven-design.md)
- **Interfaces (provided/required):** value types + published interfaces across context boundaries; application services (`ui-api` traits, [ADR 009](../../adr/009-ui-api-service-layer.md))
- **Dependencies:** C4 (the Diagram aggregate is the core context's model), C2 (Rendering & Export context hosts the pipeline)

## Concept C4: Diagram Aggregate

- **ID:** C4
- **Purpose:** the core domain aggregate — `Diagram` root with identity-bearing objects, value-object geometry, invariants enforced at the boundary; single source of truth for everything rendered/exported
- **Details:** [`doc/arc42/12-concepts/diagram-aggregate.md`](diagram-aggregate.md) · decided in [ADR 011](../../adr/011-domain-driven-design.md)
- **Interfaces (provided/required):** construction/mutation API → `Diagram` value type; consumed by rendering (C2), persistence, and import
- **Dependencies:** C3 (its bounded context), C5 (object geometry is cell-based), C2 (rendered by the pipeline)

## Concept C5: Cell Grid

- **ID:** C5
- **Purpose:** the unbounded 2D integer grid all diagram geometry lives on — `CellPos`/`CellSize` value objects, top-left origin, whole-diagram scale; one cell = one character at scale 1
- **Details:** [`doc/arc42/12-concepts/cell-grid.md`](cell-grid.md)
- **Interfaces (provided/required):** `CellPos`/`CellSize` value types + scale on the aggregate → consumed by every object kind and by rendering/export
- **Dependencies:** C4 (the aggregate owns the scale; objects carry cell geometry), C2 (rendering resolves onto this grid)

## Concept C6: Diagram File Format (`.ad`)

- **ID:** C6
- **Purpose:** the canonical lossless serialization of a `Diagram` — versioned, line-based, one statement per line with explicit cell coordinates; deterministic, byte-stable on re-save (QS7)
- **Details:** [`doc/arc42/12-concepts/diagram-file-format.md`](diagram-file-format.md) · decided in [ADR 004](../../adr/004-diagram-file-format.md); grammar settled by S-003
- **Interfaces (provided/required):** `ad-format` crate (S-004): parser + canonical serializer over the C4 aggregate
- **Dependencies:** C4 (serializes the aggregate 1:1), C5 (cell coordinates + scale), C7/C8 (`collection/name` type-ids resolve into collections)

## Concept C7: Shape Template

- **ID:** C7
- **Purpose:** a user-defined diagram object — named fixed-size grid of svgbob-alphabet characters with declared anchor cells; the unit of user content (FR3), stamped opaquely except at anchors
- **Details:** [`doc/arc42/12-concepts/shape-template.md`](shape-template.md) · decided in [ADR 007](../../adr/007-user-defined-diagram-objects.md); syntax settled by S-003
- **Interfaces (provided/required):** template definition (grid + anchors) → consumed by the Shape Catalog (C3) and rendering (C2)
- **Dependencies:** C8 (lives in collections), C6 (`collection/name` reference form), C1 (cells use the svgbob alphabet), C5 (instances occupy a cell rectangle)

## Concept C8: Collection

- **ID:** C8
- **Purpose:** a named plain-text file (`.adc`) grouping shape templates — the shareable unit of user content; identity = filename stem, `collection/name` resolution rule lives here
- **Details:** [`doc/arc42/12-concepts/collection.md`](collection.md) · decided in [ADR 007](../../adr/007-user-defined-diagram-objects.md); syntax settled by S-003
- **Interfaces (provided/required):** collection file → `shape-registry` (S-005) loads it; type-id resolution for `.ad` files (C6)
- **Dependencies:** C7 (content unit), C6 (shared versioning scheme + reference form), C3 (Shape Catalog context)
