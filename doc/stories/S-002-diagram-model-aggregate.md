---
title: S-002 - diagram-model-aggregate
description: Diagram aggregate root — value-object cell geometry, Shape/Text/Connector instances, intent-revealing mutations, invariants
scope: diagram-model
status: backlog
updated: 2026-10-22
---

# S-002: diagram-model — Diagram aggregate + cell geometry

## Why

The [Diagram aggregate](../arc42/12-concepts/diagram-aggregate.md) (C4, [ADR 011](../adr/011-domain-driven-design.md)) is the single source of truth for everything rendered/exported ([§1.1 #1](../arc42/01-introduction-and-context.md)); grid alignment in cell units is FR7 and the basis of Q1 determinism.

## Dependencies

- S-001 — workspace + gates must exist first

## Acceptance criteria

1. **AC1 — Value objects.** `CellPos`, `CellSize` (integer cell units) and `AnchorRef` are immutable value types with equality by value; construction rejects invalid geometry (e.g. zero/negative size). Check: unit tests in `diagram-model`.
2. **AC2 — Object model.** `Object::Shape(ShapeInstance{id, type_id, pos, size}) | Text(TextInstance) | Connector(ConnectorInstance{id, from, to, style})`; `Diagram` holds an ordered collection (document order = z-order) plus diagram-level properties incl. scale (FR8). Check: types + tests in `diagram-model`.
3. **AC3 — Intent-revealing mutations.** Root operations `place_shape`, `place_text`, `move_object`, `resize`, `connect`, `remove` exist and enforce invariants on every mutation: anchor cells inside their shape's bounds; connectors reference existing objects/anchor refs (no dangling refs). Check: unit tests per invariant + per mutation.
4. **AC4 — Leaf crate.** `diagram-model` has zero crate dependencies (value types only, per §8.3). Check: T5 audit test (S-001) passes unchanged.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p diagram-model` ≥ 80%.
6. **AC6 — Concept docs updated.** New concept doc `doc/arc42/12-concepts/cell-grid.md` (referenced by C1 as "not yet written") created from the template + added to §12 index and mkdocs nav; C4 (`diagram-aggregate.md`) Implementation section points at the real modules.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
