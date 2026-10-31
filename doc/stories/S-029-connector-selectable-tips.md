---
title: S-029-connector-selectable-tips
description: CR-008 — selectable per-end connector tips + arrow-shaped anchor markers (FR9)
scope: diagram-model/ad-format/render-pipeline/gui-app
status: done
updated: 2026-10-31
---

# S-029: render-pipeline + gui-app — selectable per-end tips + arrow anchor markers (FR9)

## Why

FR9 extended by CR-008: each connector end carries a selectable tip style — none / hollow arrow / empty rhombus `◇` / filled rhombus `◆` — changed by pressing onto the tip; files without tip data keep the legacy default (source none, target arrow) so existing renders stay byte-identical. GUI anchor dots become small arrow symbols signalling "connecting adds an arrow". ADR 013 v5; rhombi are content-only glyphs (no ADR 001 bump).

## Dependencies

- S-028 — same module; tip hit-testing sits on the route

## Acceptance criteria

Written from [CR-008](../crs/CR-008-selectable-arrow-tips.md)'s "Requested change" + the §14 FR9 wording, per ADR 013 v5 (point 5) and concept C9.

- **AC1 — model:** `ConnectorInstance` carries a tip style per end (`from_tip`, `to_tip`) from {none, hollow arrow, empty rhombus, filled rhombus}; the aggregate exposes `set_tip(id, end, style)` with typed errors for unknown connectors and non-connectors. Defaults are source = none, target = hollow arrow — every existing diagram renders identically (NFR2).
- **AC2 — format:** the `.ad` `connect` statement accepts an optional `tip <src> <tgt>` clause (keywords `none` / `arrow` / `rhombus-open` / `rhombus-filled`) between `dashed` and `via`; the serializer emits it only when the pair differs from the defaults; round-trip is byte-stable; files without the clause parse with the legacy defaults; a malformed clause is a line/column parse error.
- **AC3 — pipeline:** each end stamps its chosen glyph as content on the cell adjacent to that endpoint (target: second-to-last route cell, source: second), pointing along the outer segment for hollow arrows (`< > ^ v`); `◇`/`◆` are direction-independent; none stamps nothing. With exactly three route cells both tips would share the middle cell — the target wins. Tip-free connectors render byte-identically to before (all pre-existing goldens unchanged); a new golden pins all four styles on two routes.
- **AC4 — GUI cycle:** pressing onto a connector's tip cell cycles that end through the four styles live (bare → arrow → empty rhombus → filled rhombus → bare), selects the connector, and reports the change in the status bar. Tip cells are hit-tested below bends/anchors/shapes (a declared bend on the same cell keeps winning).
- **AC5 — GUI anchor markers:** the selection's anchor points are drawn as small arrow glyphs pointing outward from each edge (`<` west, `>` east, `^` north, `v` south; interior/custom anchors fall back to `·`) instead of plain dots — signalling that dragging from an anchor adds an arrow. Anchor hit-testing and the connect gesture are unchanged (WYSIWYG: the markers are overlay-only and never enter the resolved grid).

## Implementation notes

- Model: `TipStyle` + `ConnectorEnd` enums with `Display` (status-bar wording); `ConnectorInstance { from_tip, to_tip }` defaulting to `(Bare, HollowArrow)`; builder `with_tips(from, to)`; `Diagram::set_tip` reuses the existing `UnknownObject` / `NotAConnector` errors.
- Format: `tip <src> <tgt>` is the third optional clause of `connect` (after `dashed`, before `via`) — two style keywords, line/column errors for truncated/unknown values and clause-order violations; serializer omits the clause at defaults so legacy files stay byte-identical.
- Pipeline: `tip_glyph(style, toward)` maps styles to content glyphs; stamped after edge/diagonal stamping (content over edges) on the adjacent cells; the three-cell collision rule keeps the target arrow visible.
- Seam: unchanged — tips are model data flowing through the existing `connector_routes` seam; no new ui-api surface.
- GUI: `hit_target` reports `HitTarget::Tip { id, end }` for the two tip cells (checked after bends, before body segments); editor gesture = one-shot cycle on press (`cycle_tip`, no drag state); canvas overlays replace `AnchorDot` with `AnchorArrow { cell, glyph }` computed by `anchor_glyph(position, size)`; app.rs draws each arrow as centered monospace text in the anchor color.

## Verification

All 5 ACs verified — workspace fmt/clippy(0)/test green; tarpaulin **94.00%** ≥ 80%.

- **AC1 — model:** `TipStyle`/`ConnectorEnd` + `Display` (`diagram-model::objects`); defaults pinned by `connectors_start_with_default_tips`; `Diagram::set_tip` unit-tested incl. unknown id and non-connector targets (`set_tip_updates_the_addressed_end_and_reports_typed_errors`).
- **AC2 — format:** `parse_tip_style` + clause wiring in the `connect` parser; serializer emits `tip` only off-defaults. Pinned: `tip_clause_parses_per_end_styles`, `tip_defaults_when_clause_absent`, `tip_clause_errors_are_positioned`, `tip_styles_serialize_and_round_trip` (byte-stable), `tip_defaults_are_not_emitted`.
- **AC3 — pipeline:** `tip_glyph` + adjacent-cell stamping in `place_connector`. Pinned: `tip_glyphs_cover_all_styles`, `source_tip_stamps_second_cell_pointing_along_first_segment`, `rhombus_tips_stamp_the_cells_adjacent_to_each_endpoint`, `bare_target_tip_stamps_nothing`, `coincident_tip_cells_the_target_wins`; new golden `connector-tips.grid` (hollow arrow + filled rhombus on the horizontal route, empty rhombus + bare end on the vertical one). All pre-existing goldens stayed byte-identical.
- **AC4 — GUI cycle:** `hit_target` reports `Tip { id, end }` for the adjacent cells of each non-bare end (bends keep priority). Pinned: `hit_target_reports_tip_cells_at_the_route_ends`, `bent_route_tip_cells_sit_on_the_outer_segments`; tier-0b `pressing_the_connector_tip_cycles_its_style` — real UI: two boxes + dragged connector, three presses on the target tip cell; each press asserts the status-bar wording ("target tip set to …") and the model's `to_tip`, and the final bare state leaves no rhombus glyph drawn.
- **AC5 — GUI anchor markers:** `Overlay::AnchorArrow` + `anchor_glyph` (edge → outward arrow, interior → `·`); app.rs draws centered text in the anchor color. Pinned: `anchor_arrows_point_outward_from_the_selection_edges` (tier-0b — the four glyphs at the exact edge-midpoint cells of a selected box) + canvas overlay unit tests for connecting/selected states.

New tests this story: 2 model + 5 ad-format + 5 pipeline unit + 1 golden scenario + 2 canvas unit + 1 tier-0b (plus one rewritten anchor-marker test and one b002 baseline fix — anchor glyphs are text now).
