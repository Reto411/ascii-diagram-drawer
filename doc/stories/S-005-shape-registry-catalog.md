---
title: S-005 - shape-registry-catalog
description: Shape Catalog context — built-in shapes with standard anchor sets, template/collection file loading, collection/name resolution
scope: shape-registry
status: done
updated: 2026-10-25
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

- Leaf crate per the §8.3 dependency audit: zero workspace dependencies; registry-local value types (`CellPos`, `CellSize` in `geometry.rs`) instead of importing `diagram-model`.
- Modules: `types` (`TemplateGrid`, `AnchorSet`, `ShapeType`, `Collection`), `builtin` (catalog as data), `templates` (`.adc` parser — mirrors `ad-format` conventions: 1-based line/column errors, canonical + lenient header forms, unknown-statement preservation only for higher minor versions, statement order kept in `Slot`s for canonical re-save), `registry` (`ShapeRegistry`, disjoint namespaces), `error` (typed errors incl. `ParseError { line, column, message }`).
- Built-ins: `box` 9×5 rectangle and `ellipse` 9×5 svgbob-style oval (radius-4 art from the svgbob spec), both with the standard four edge-midpoint anchors n/s/w/e — recorded in the `shape-template.md` concept doc.

## Verification

- [x] AC1 — `ShapeType { type_id, name, glyph_pattern, anchor_set, resize_rules }` + `Collection { name, types }`; box + ellipse defined as data in `builtin.rs`; tests assert each built-in's exact anchor set and size (`box_anchor_set_is_the_mid_side_points`, `ellipse_anchor_set_is_the_mid_side_points`).
- [x] AC2 — `parse_collection` implements the S-003 grammar (grid rows right-padded, printable-ASCII-only cells, coordinate-based anchors); 14 invalid-file cases in `invalid_files_produce_location_errors` (≥4 required), all with line numbers, no panics.
- [x] AC3 — `ShapeRegistry::lookup`: bare id → built-ins only, `collection/name` → user collections only, disjoint namespaces (no fallback/shadowing); typed `LookupError::{UnknownBuiltin, UnknownCollection, UnknownTemplate}`; one test per resolution path.
- [x] AC4 — v1 template shapes carry `resize_rules: None` (asserted in `every_builtin_has_standard_anchors_and_no_resize_rules` and the `.adc` parse tests); the field stays on the type.
- [x] AC5 — fmt/clippy clean under ADR 010 (`clippy::all` deny); 32 crate tests green; workspace tarpaulin (Llvm) 97.58% ≥ 80%; shape-registry files 391/408 ≈ 96%.
- [x] AC6 — `shape-template.md` + `collection.md` Implementation sections point at the real modules with the standard default anchor set recorded; overview tracker updated.
