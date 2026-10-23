---
title: diagram-aggregate
description: the core domain aggregate — Diagram root with identity-bearing objects, value-object geometry, invariants enforced at the boundary; single source of truth for everything rendered/exported
scope: model
status: stable
updated: 2026-10-23
---

# Concept: Diagram Aggregate

## Definition

The `Diagram` is the **aggregate root** of the core domain: an ordered collection of objects (shapes, text, connectors) placed on a cell grid. Document order = z-order. Everything that gets rendered or exported derives from this one value; nothing outside it may bypass its invariants.

## Why it exists

[ADR 011](../../adr/011-domain-driven-design.md) introduced the tactical DDD structure for [§8](../../arc42/08-building-block-view.md). A single aggregate with boundary-enforced invariants makes invalid states unrepresentable (no off-grid coordinates, no dangling connectors), which is what Q1 determinism and the WYSIWYG promise rest on: a well-formed model → one resolved grid → identical text/SVG/PNG.

## How it works / behaves

- **Entities** (stable identity): `Object::Shape(ShapeInstance) | Text(TextInstance) | Connector(ConnectorInstance)`.
  - `ShapeInstance { id, label, type_id, pos: CellPos, size: CellSize }` — coordinates in **cell units** (grid alignment, FR7); `type_id` is the only reference into the Shape Catalog context; `label` renders inside the shape.
  - `ConnectorInstance { id, from: AnchorRef, to: AnchorRef, style }` — bound to anchors, not raw coordinates (FR9).
- **Value objects:** `CellPos`, `CellSize`, `AnchorRef` — immutable geometry, equality by value.
- **Invariants** (enforced at construction *and* on every mutation — see `ModelError` for the failure surface): unique object ids; single-token ids/type-ids/anchor names (no whitespace); non-zero sizes (`CellSize ≥ 1×1`); scale ≥ 1; connector endpoints reference existing objects that are shapes. Invariants needing the shape catalog (an anchor name resolving to a cell inside its shape's bounds) are checked at render time in `render-pipeline`, which alone sees both sides.
- **Mutations:** intent-revealing operations on the root (`place`, `move_object`, `resize`, `connect`, …). The GUI manipulates the aggregate directly — deliberately no "model service" trait ([ADR 009](../../adr/009-ui-api-service-layer.md) non-abstraction); there is no anemic-model field poking (§8.6).
- **Lifecycle:** created by user gestures, by `.ad` parsing (Repository), or by ASCII import (factory); mutated in place per gesture; serialized losslessly back to `.ad`.

## Relations

- **Bounded contexts** — the aggregate is owned by the Diagram context (link: `doc/arc42/12-concepts/bounded-contexts.md`)
- **Render pipeline** — consumes the aggregate (with the catalog) to produce the resolved grid (link: `doc/arc42/12-concepts/render-pipeline.md`)
- Persistence (`.ad`) serializes this exact aggregate — no separate format model ([ADR 004](../../adr/004-diagram-file-format.md))

## Implementation

The `diagram-model` crate (leaf: zero dependencies), per [§8.4.1](../../arc42/08-building-block-view.md):

- `crates/diagram-model/src/lib.rs` — the `Diagram` aggregate root: `objects` in document order (= z-order) + `scale`; intent-revealing mutations (`place_shape`, `place_text`, `connect`, `remove`, `move_object`, `resize`, `set_scale`) that re-check the boundary invariants.
- `crates/diagram-model/src/objects.rs` — `ObjectId`, `Object` enum, `ShapeInstance`, `TextInstance`, `ConnectorInstance`, `ConnectorStyle`.
- `crates/diagram-model/src/anchors.rs` — `AnchorRef { object, anchor }` (validated single-token anchor name).
- `crates/diagram-model/src/invariants.rs` — `ModelError` (the whole failure surface) + shared token validation.
- Geometry value objects live in the [cell grid](cell-grid.md) concept (`geometry.rs`).
