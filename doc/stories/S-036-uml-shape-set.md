---
title: S-036-uml-shape-set
description: CR-015 — core UML shape set in the built-in catalog (FR18) (skeleton; ACs at implementation)
scope: shape-registry
status: done
updated: 2026-11-01
---

# S-036: shape-registry — core UML shape set in the built-in catalog (FR18)

## Why

FR18: the requester is a system architect who draws UML daily. The built-in catalog ships the core UML shape set for class/use-case/sequence/component/deployment/state/activity diagrams (at minimum interface lollipop, actor, activation bar, component, port, deployment node, artifact, rounded-rectangle state, decision diamond, initial/final state nodes) — each a built-in with standard anchors (FR11), verbatim glyph art in the svgbob alphabet. Geometry only; UML semantics come from shape-owned text boxes (S-037/FR19) and free text (FR10). Exact glyph art + per-shape anchor sets finalized in this story.

## Dependencies

- none beyond done stories (S-005); compartment text arrives with S-037

## Acceptance criteria

> Written from [CR-015](../crs/CR-015-uml-shape-set.md)'s "Requested change" + §14 FR18 wording when work started.

- **AC1 (FR18 shape set):** `builtin_catalog()` contains, in addition to `box` and `ellipse`, at minimum the core UML set: interface lollipop, actor, activation bar, component, port, deployment node, artifact, rounded-rectangle state, decision diamond, initial state node, final state node — each a `ShapeType` with a unique bare type-id. Pinned by an exact-name test.
- **AC2 (FR11 standard anchors):** every new built-in declares exactly the four standard anchors `n`/`s`/`w`/`e` (mid-side / UML connection points) and no others; where a midpoint falls on drawn art the anchor cell is a drawn (non-space) cell, and the deliberate exceptions (decision diamond's implied top/bottom points, actor's between-feet point) are pinned by name and position.
- **AC3 (verbatim glyph art):** every new built-in is `GeometryKind::Pattern` with no resize rules and printable-ASCII-only art; a pipeline test renders a diagram containing the UML set plus connectors bound to their anchors and asserts the exact grid — art stamps byte-for-byte at natural size, connectors bind.
- **AC4 (UML semantics via text boxes):** the shapes with an interior compartment — component, deployment node, artifact, state — declare exactly one `name` region covering their interior (rects pinned); the remaining UML shapes stay label-only. Double-clicking a placed compartmented built-in opens S-037's region-content window with its `name` field (tier-0b test).
- **AC5 (seam):** the new built-ins flow through `ShapeRegistry::builtin()` and the ui-api catalog seam in deterministic order with `resizable = false`, `min_size` = natural size, and declared regions exposed as `RegionInfo` — so the GUI palette previews/list them automatically via existing S-016/S-032/S-034 behavior (pinned by a core-services test).

## Implementation notes

Landed in the owning bounded context (ADR 011) — catalog data plus tests, no pipeline or seam change:

- **shape-registry** — `builtin_catalog()` gains the eleven core UML shapes: interface lollipop, actor, activation bar, component, port, deployment node, artifact, rounded-rectangle state, decision diamond, initial and final state nodes. Every one is `GeometryKind::Pattern` with no resize rules (stamped byte-for-byte at natural size; non-resizable through the seam per the S-024 rule), printable-ASCII-only art in the svgbob alphabet, and exactly the four standard anchors `n`/`s`/`w`/`e` placed on UML connection points. Where a midpoint falls on drawn art the anchor cell is a drawn cell; the deliberate exceptions are pinned by name — `decision.n`/`decision.s` (the diamond's implied apex/bottom points) and `actor.s` (between the feet) sit on space cells and bind live once a connector attaches. The compartmented shapes (component, deployment node, artifact, state) declare exactly one `name` region covering their interior via S-037's mechanism (FR19); every other UML shape stays label-only.
- **render-pipeline** — new integration test `tests/uml_set.rs`: a 38×30 diagram with all eleven shapes plus two anchor-bound connectors (a straight segment and an L through the row gap), asserting the exact grid. Only the four connector-endpoint cells deviate from the templates (junction `+` over their anchor cell, as in the pre-existing resize tests).
- **core-services** — seam test pins all 13 built-ins across the ui-api catalog in deterministic (alphabetical) order with natural size, verbatim preview, declared anchors, `resizable = false` / `min_size` = natural size for the no-rule Pattern shapes (box/ellipse stay resizable to 1×1), and declared regions exposed as `RegionInfo`.
- **gui-app** — the sidebar's pre-rendered previews (S-034) pick up the new catalog automatically; material goldens regenerated. New helper `scroll_sidebar_until` scrolls a sidebar label fully above the bottom status bar with sub-smoothing point-unit wheel deltas — the full 13-shape catalog pushes groups below the 768 px fold, so tests (like users) must scroll to reach them.

## Verification

- **AC1** — `shape-registry`: `catalog_is_deterministic_and_contains_the_core_uml_set` (exact names + deterministic order); the registry's built-in name list test now pins all 13 ids.
- **AC2** — `shape-registry`: `uml_anchor_positions_are_pinned` (exactly `n`/`s`/`w`/`e` per shape, positions pinned, drawn-art cells asserted, the deliberate space-cell exceptions pinned by name).
- **AC3** — `shape-registry`: `uml_shapes_are_verbatim_patterns` (Pattern kind, no rules, printable ASCII only) + `uml_shapes_carry_their_exact_glyph_art`; `render-pipeline`: `uml_set_stamps_verbatim_and_connectors_bind_to_anchors` (exact 38×30 grid — art byte-for-byte, connectors bound to their anchors).
- **AC4** — `shape-registry`: `compartmented_builtins_declare_exactly_one_name_region` (exact interior rects; every other built-in label-only); `gui-app` tier-0b: `double_click_builtin_component_opens_name_region_editor` (place the built-in component from the sidebar → double-click → S-037's window opens with its `name:` field).
- **AC5** — `core-services`: `uml_set_crosses_the_seam_with_correct_metadata` (deterministic order, size/preview/anchors/resizable/min_size/regions for all 13 built-ins).
- **Gates** — `cargo fmt` clean; `cargo clippy --workspace --all-targets` zero errors (default group denied); full suite green (653 tests); `cargo tarpaulin --engine Llvm --workspace` = 92.93% (≥ 80% target). Material goldens regenerated — the sidebar now previews all 13 built-ins. 
