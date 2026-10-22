---
title: S-005 - shape-registry-catalog
description: Shape Catalog context — built-in shapes with standard anchor sets, template/collection file loading, collection/name resolution
scope: shape-registry
status: backlog
updated: 2026-10-22
---

# S-005: shape-registry — built-in catalog + template/collection loading

## Why

The Shape Catalog context ([§8.4.2](../arc42/08-building-block-view.md)) owns `ShapeType`, glyph patterns, anchor sets and collections ([ADR 007](../adr/007-user-defined-diagram-objects.md)); FR3 (user objects) and FR11 (per-type anchor sets with predefined defaults) live here. The diagram carries only `type_id`s — meaning is resolved through this catalog at render time (C3 context mapping).

## Dependencies

- S-002 — anchor refs / geometry value types from the core model
- S-003 — template + collection file syntax to parse

## Acceptance criteria

1. **AC1 — Types.** `ShapeType { type_id, name, glyph_pattern: TemplateGrid, anchor_set: AnchorSet, resize_rules: Option<_> }` and `Collection { name, types }`; built-in shapes (at minimum box + circle/ellipse) defined as data with standard default anchors (basic edge positions per FR11 — the chosen default set is recorded in the `shape-template.md` concept doc). Check: unit tests asserting each built-in's anchor set.
2. **AC2 — Template/collection loading.** Load template + collection files from text per the S-003 grammar: glyph grid parsed (svgbob-alphabet characters only), anchors marked per the marker rule; invalid files → error with location, never a panic. Check: unit tests incl. ≥4 invalid-file cases.
3. **AC3 — Resolution.** `lookup("collection/name")` and bare built-in names resolve to `ShapeType`; unknown IDs → typed error (feeds RS2-style notices later). Check: unit tests per resolution path from ADR 007.
4. **AC4 — v1 fixed-size rule.** All template shapes have `resize_rules = None` (ADR 007 v3); the type still carries the field for future resize rules. Check: code + test asserting custom templates are fixed-size.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p shape-registry` ≥ 80%.
6. **AC6 — Concept docs updated.** `shape-template.md` + `collection.md` Implementation sections point at real modules (default anchor set recorded); overview tracker updated.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
