---
title: S-028-connector-manual-bends
description: CR-007 — manual bend waypoints for connectors (FR9)
scope: diagram-model/ad-format/render-pipeline/gui-app
status: done
updated: 2026-10-31
---

# S-028: render-pipeline + gui-app — manual bend waypoints for connectors (FR9)

## Why

FR9 extended by CR-007: automatic routing cannot always produce a readable layout (routes crossing shapes/labels). Pressing an arrow selects it; dragging adds/moves intermediate bends (waypoints) so arrows can be routed manually (Visio-style); bend-free connectors keep automatic routing. Route = polyline through waypoints — axis-aligned segments structural, explicit diagonals `/` `\` content (ADR 013 v4).

## Dependencies

- none beyond done stories (S-007)

## Acceptance criteria

Written from [CR-007](../crs/CR-007-manual-arrow-bends.md)'s "Requested change" + the §14 FR9 wording, per ADR 013 v4 (point 3) and concept C9.

- **AC1 — model:** `ConnectorInstance` carries an ordered list of optional manual waypoints (`bends`, model-space cells); the aggregate exposes `add_bend` / `move_bend` / `remove_bend` with typed errors for unknown connectors or out-of-range indices. Connectors without bends keep the existing default (empty list) and render exactly as before.
- **AC2 — format:** the `.ad` `connect` statement accepts an optional `via <x> <y>, …` waypoint list (model-space cells, C6 grammar); the serializer emits it only when non-empty; round-trip is byte-stable; files without `via` parse with empty bends; a malformed `via` list is a line/column parse error.
- **AC3 — pipeline:** when `bends` is non-empty the route is the polyline `[end₁, bends…, end₂]`; axis-aligned segments stamp structural edges (clean C1 joins); a segment whose endpoints share neither row nor column is a straight diagonal stamped as `/` or `\` content. Bend-free connectors keep automatic routing — all existing connector goldens stay byte-identical. New goldens pin an orthogonal bent route and a diagonal segment; routing stays a pure function of resolved endpoints + declared bends (NFR3).
- **AC4 — GUI selection:** pressing onto a connector's body selects it (`HitTarget::ConnectorBody`); pressing onto a waypoint cell hits the bend itself (`HitTarget::Bend`). Connectors are hit-tested *below* shapes and text — anchors, resize handles and object bodies keep priority.
- **AC5 — GUI add/move gestures:** dragging from a connector body adds a new bend at the dropped cell (inserted between the pressed segment's endpoints); dragging an existing bend moves it live through `move_bend`. While dragging, a waypoint snaps to the row or column of either adjacent point whenever the pointer is on that axis — axis-aligned segments stay axis-aligned — and a diagonal only appears when the pointer is off both axes of both neighbors (ADR 013 v4).
- **AC6 — GUI removal:** dropping a moved bend exactly on the straight line between its two adjacent points (collinear, including coinciding with one of them) removes it (Visio-style cleanup); a press without movement adds nothing and moves nothing.
- **AC7 — WYSIWYG + handles:** the selected connector draws a visible handle at each waypoint; the canvas keeps rendering the exact resolved-grid bytes, so bends appear identically in text/SVG/PNG output and the preview (no separate GUI drawing path).

## Implementation notes

- Model: `bends: Vec<CellPos>` on `ConnectorInstance` (`diagram-model::objects`); aggregate ops `Diagram::{add_bend, move_bend, remove_bend}` — bends are plain coordinates, no referential integrity needed.
- Format: `via` is the last optional clause of `connect` (after `dashed`), so S-029's tip keywords can slot in before it; waypoints are bare non-negative integers, comma-separated, model-space cells.
- Pipeline: `route()` gains a bend-aware branch — polyline through `[from, bends…, to]`; per consecutive pair: axis-aligned → existing structural-edge stamping; otherwise Bresenham straight diagonal stamped as `/` (rising right / falling left) or `\` content. Arrowhead stays on the second-to-last route cell; a diagonal final segment picks its glyph by dominant direction.
- Seam: `ui_api::Renderer` gains `connector_routes(diagram) -> Result<Vec<(ObjectId, Vec<CellPos>)>, RenderError>` (real backend delegates to the pipeline; `FakeRenderer` returns configurable routes) — gui-app must not link render-pipeline directly (§8.3).
- GUI: `hit_target` takes the per-connector route cells and reports `Bend { id, index }` / `ConnectorBody { id, segment }` (segment = insertion point for a new bend); editor gains a `BendDrag` gesture implementing AC5/AC6 snapping + collinear removal; app bar/canvas draw waypoint handles for the selected connector.
- Snapping interpretation (recorded per ADR 013 v4's latitude "the GUI constrains bend editing…"): free 2-D drag with live axis snap to the two adjacent points' rows/columns — gentle drags stay orthogonal, diagonals require the pointer off both axes of both neighbors.

## Verification

All 7 ACs verified — workspace fmt/clippy(0)/test green; tarpaulin **94.00%** ≥ 80%.

- **AC1 — model:** `ConnectorInstance.bends: Vec<CellPos>` (`diagram-model objects.rs`); aggregate ops `Diagram::{add_bend, move_bend, remove_bend}` with typed `ModelError::{UnknownObject, NotAConnector, BendIndexOutOfRange}` (unit-tested incl. unknown id / non-connector / out-of-range). Default is an empty list — all five pre-existing connector goldens stayed byte-identical (only two new golden files added).
- **AC2 — format:** `connect … [dashed] [via <x> <y>, …]` — `parse_via_list` (line/column errors for truncated pairs, trailing comma, non-integers; clause-order error naming the rule); serializer emits `via` only when non-empty. Pinned: `via_waypoints_parse_into_bends`, `via_list_errors_are_positioned`, `dashed_without_via_keeps_empty_bends`, `via_bends_serialize_and_round_trip` (byte-stable round-trip).
- **AC3 — pipeline:** `connector_routes` computes the polyline `[end₁, bends…, end₂]`; axis-aligned segments stamp structural edges (clean C1 joins), oblique segments are integer-Bresenham straight lines stamped `/` (rising right) or `\` (falling right) content; bend-free connectors keep automatic routing. New goldens: `connector-bent.grid` (vertical run + `\` diagonal + horizontal final segment with arrowhead) and `connector-bent-diagonal-first.grid` (diagonal first segment — departure cell keeps structural edges). Routing is a pure function of resolved endpoints + declared bends (NFR3; proptest suite unchanged and green).
- **AC4 — GUI selection:** `hit_target` gained per-connector route cells: bend cells hit `Bend { id, index, prev, next }` (a bend wins over its two segments), segment cells hit `ConnectorBody { id, segment, prev, next }` with the pressed segment's endpoints (the insertion point); connectors are tested *after* shapes/text/anchors so object priority is untouched. Pinned: `hit_target_reports_a_bend_on_its_cell`, `hit_target_reports_body_cells_with_segment_endpoints`, `hit_target_shapes_win_over_connector_bodies`, `hit_target_ignores_connectors_off_route_and_while_connecting`.
- **AC5 — GUI add/move:** editor gestures `BendMove` (live `move_bend` on every cell crossing) and `BendAdd` (created at first displacement, inserted at the pressed segment, then moved live). On the discrete grid the AC5 axis-snap rule is exact: sharing a row/column with a neighbor keeps that segment axis-aligned automatically; a diagonal to a neighbor appears exactly when the pointer is off both of its axes (documented in code — no separate snap function needed). Pinned: `pressing_a_bend_selects_the_connector_and_moves_it_live`, `pressing_a_connector_body_and_dragging_adds_a_bend_at_the_drop`, `bend_dragging_is_free_with_exact_axis_alignment`.
- **AC6 — GUI removal:** dropping a *moved* bend on the straight line between its neighbors (`canvas::on_segment`) removes it via `remove_bend`; a press without movement never adds/removes. Bug found & fixed by the tests: the BendAdd "still on the pressed cell" guard must not block a live move back over the press point after the bend exists. Pinned: `dropping_a_moved_bend_on_the_neighbor_line_removes_it`, `pressing_a_bend_without_moving_changes_nothing`, `dragging_from_a_body_and_dropping_back_on_the_line_adds_nothing`.
- **AC7 — WYSIWYG + handles:** the canvas keeps drawing the exact resolved-grid bytes (no separate GUI path for bends); the selected connector gets a visible `Overlay::BendHandle` per waypoint, drawn by the existing overlay pass (`app.rs`) and hit-tested back to the same cell. Pinned: `overlays_show_bend_handles_only_for_the_selected_connector` + geometry tests `on_segment_axis_aligned`, `on_segment_diagonal_walks_the_bresenham_line`, `on_segment_degenerate_point`.

Seam per ADR 009/§8.3: `ui_api::Renderer::connector_routes(diagram) -> Result<Vec<(ObjectId, Vec<CellPos>)>, RenderError>` — the real backend delegates to `render_pipeline::connector_routes`; `FakeRenderer` serves configurable routes so gui-app never links render-pipeline. New tests this story: 3 model + 5 ad-format (incl. the serializer round-trip) + 2 pipeline goldens + 8 canvas unit + 6 editor gesture; tier-0b suite unchanged and green. 
