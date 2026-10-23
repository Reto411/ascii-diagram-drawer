---
title: S-002 - diagram-model-aggregate
description: Diagram aggregate root — value-object cell geometry, Shape/Text/Connector instances, intent-revealing mutations, invariants
scope: diagram-model
status: done
updated: 2026-10-23
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

- `diagram-model` is a leaf crate (zero `[dependencies]`); geometry/objects/anchors/invariants live in their own modules per §8.4.1, with the `Diagram` aggregate root in `lib.rs`.
- **Invariant split (doc fix in §8.4.1 + C4):** §8.4.1 originally listed "anchor cells inside their shape's bounds" as a boundary-enforced invariant, but that check needs the shape catalog (`AnchorSet` lives in `shape-registry`, which depends *on* diagram-model — the reverse direction is impossible for a leaf crate). Consistent with §8.2.3 ("full meaning … resolved through the catalog at render time"), boundary-enforced invariants are: unique ids, single-token ids/type-ids/anchor names, non-zero sizes, scale ≥ 1, connector endpoints exist and are shapes. The anchor-geometry invariant is enforced at render time in `render-pipeline` (S-006/S-007). §8.4.1 and C4 updated accordingly; no ADR needed — it follows directly from ADR 011's context mapping + AC4.
- `remove` cascades to referencing connectors so endpoints never dangle; `new_object_id` is deterministic (`obj-{n}`, skips used numbers) for Q1.
- Seven new workspace-level Clippy exceptions documented in [ADR 010](../adr/010-deny-all-clippy-lints.md) v3 (first real code exposed them: ordering, const-fn, `core` imports, exhaustive types, …).

## Verification

- [x] AC1 — `CellPos`/`CellSize`/`AnchorRef` are immutable value types (`PartialEq` by value, derived); zero sizes rejected at construction (`ModelError::ZeroSize`), negative coordinates unrepresentable (unsigned). Unit tests in `geometry.rs`, `anchors.rs`.
- [x] AC2 — `Object::Shape|Text|Connector` with instances per the spec (shape incl. `label`, see ADR 004); `Diagram` holds an ordered `Vec<Object>` (document order = z-order) + `scale`. Tests in `objects.rs`, `lib.rs`.
- [x] AC3 — `place_shape`, `place_text`, `move_object`, `resize`, `connect`, `remove`, `set_scale` all enforce the boundary invariants on every mutation (duplicate ids, unknown objects, non-shape connector endpoints, unmovable connectors, zero scale); each invariant + mutation has a unit test. Anchor-geometry half of the criterion is render-time by design — see Implementation notes.
- [x] AC4 — `crates/diagram-model/Cargo.toml` has an empty `[dependencies]`; T5 audit (core-services) passes unchanged.
- [x] AC5 — `cargo fmt --check` clean; `cargo clippy -p diagram-model --all-targets` 0 errors; 30/30 tests pass; tarpaulin (Llvm) **175/175 lines = 100%** (≥ 80% floor).
- [x] AC6 — new concept doc [`cell-grid.md`](../arc42/12-concepts/cell-grid.md) from the template, added to §12 index (C5) + mkdocs nav; C4 `diagram-aggregate.md` Implementation section now points at the real modules; C4 status draft → stable. 
