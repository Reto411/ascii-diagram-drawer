---
title: S-031-connector-free-endpoints
description: CR-010 — standalone arrows, attachable/detachable ends (FR9): model endpoint sum type, .ad per-endpoint encoding, pipeline free-end resolution, GUI arrow tool + endpoint-drag gesture
scope: diagram-model/ad-format/render-pipeline/gui-app
status: done
updated: 2026-10-31
---

# S-031: model + ad-format + render-pipeline + gui-app — standalone arrows, attachable/detachable ends (FR9)

## Why

FR9 extended by CR-010: arrows whose ends are not bound to any shape (annotation arrows, flows between regions). Each end is either anchor-bound (follows the shape) or free at an explicit model-space position; attach/detach converts between the forms interactively. Zero/one/two bound ends are legal. ADR 013 v6; endpoint syntax joins the `.ad` 1.1 batch.

## Dependencies

- S-028/S-029 — endpoint manipulation UI sits on the route (both done)

## Acceptance criteria

Written from [CR-010](../crs/CR-010-standalone-arrows.md)'s "Requested change" + §14 FR9 wording, plus ADR 013 v6 point 7.

1. **Model — endpoint sum type.** `diagram-model` gains `ConnectorEndpoint { Attached(AnchorRef), Free(CellPos) }`; `ConnectorInstance.from`/`.to` carry it. A connector with zero, one, or two bound ends is legal. Both-ends-free at the *same* cell (zero length) is rejected by the aggregate (`ModelError`) — CR-010 assumption "minimum 1 cell". Attached ends keep today's validation (referenced object exists and is a shape).
2. **Model — attach/detach operation.** `Diagram::set_endpoint(id, end, endpoint)` converts one end between the two forms; it validates attached targets exactly like `connect` and re-applies the zero-length rule. Deleting a shape keeps today's cascade behavior for connectors bound to it (unchanged).
3. **Format — per-endpoint encoding.** `.ad` `connect` statements accept either form per end: attached `<obj>.<anchor>` (today's syntax, unchanged) or free `<x> <y>` (two bare non-negative integers). Disambiguation is by token shape: an attached endpoint always contains a dot, a free one never does — no keyword added. Round-trip is byte-stable for both forms; files without free endpoints serialize exactly as before (existing goldens stay green).
4. **Pipeline — free-end resolution.** The render pipeline resolves `Free(pos)` to its stored cell with no shape lookup (`side: None` — no side preference for corner choice); routing, tips and bends are otherwise unchanged (ADR 013 v6: the route stays a pure function of two cells). New goldens pin a standalone arrow (both ends free) and a mixed attached-free connector.
5. **GUI — "Arrow" tool (standalone creation).** The sidebar gains an Arrow tool. Press-drag-release on the canvas creates a connector whose source end is at the press cell and target end at the release cell; an end is `Attached` when its start/drop cell is exactly a shape anchor, otherwise `Free`. Both-ends-free on one cell is refused (criterion 1). The existing anchor-to-anchor connect gesture (S-013) is unchanged.
6. **GUI — endpoint drag (attach/detach).** Pressing a connector's endpoint cell (or its tip cell) starts an endpoint-drag gesture; pressing without moving still cycles the tip style (S-029 behavior preserved). While dragging, the end follows the pointer: a free end moves live (`set_endpoint(Free)`), an attached end is detached the moment it leaves its anchor cell. Releasing over a shape anchor re-attaches (snaps) that end; releasing elsewhere leaves it free at the drop cell. During arrow-drawing and endpoint-dragging, every shape's anchors are exposed as snap targets (same mechanism as the connect drag).

## Implementation notes

- `diagram-model`: `ConnectorEndpoint` in `anchors.rs`; `ConnectorInstance::new` takes endpoints; `Diagram::connect` validates per-end + zero-length rule; new `set_endpoint`.
- `ad-format`: `parse_endpoint` (dot ⇒ attached, else two ints); serializer emits `x y` for free ends.
- `render-pipeline`: `resolve_endpoint` matches on the sum type; proptest generators gain free endpoints.
- `gui-app`: `HitTarget::Endpoint { id, end }` (hit below selected-shape anchors/resize handle, above bodies — S-013 connect-from-anchor is untouched); `AnchorExposure` generalizes the old `connecting_from` parameter of `hit_target` (`SelectionOnly | Connect{source} | All`); `Placing::Arrow`; gestures `ArrowDrag` / `EndpointDrag` in `editor.rs`; `PointerEvent::Up` now carries the release cell (needed for free drops); new `EditOutcome`s: `ArrowPlaced`, `EndpointAttached`, `EndpointDetached`, `EndpointMoved`.
- Attach rule is exact-cell (drop cell == anchor's resolved cell) — same precision as the S-013 connect drop; no fuzzy radius on a discrete grid.

## Verification

All 6 ACs verified — workspace fmt/clippy(0)/test green (547 passed); tarpaulin **93.69%** ≥ 80%; mkdocs --strict clean.

- **AC1 — endpoint sum type:** `ConnectorEndpoint { Attached(AnchorRef), Free(CellPos) }` in `diagram-model::anchors`; `ConnectorInstance.from`/`.to` carry it. Pinned: `endpoint_forms_expose_their_parts`, `connect_accepts_free_ends_and_rejects_zero_length` (zero/one/two bound ends legal; both-free on one cell → `ModelError`), `connect_requires_existing_shape_endpoints` (attached validation unchanged).
- **AC2 — attach/detach:** `Diagram::set_endpoint(id, end, endpoint)` converts between the forms with full re-validation. Pinned: `set_endpoint_attaches_detaches_and_enforces_invariants` (attach to missing object / non-shape / zero-length result all rejected; detach + re-attach round-trips).
- **AC3 — per-endpoint encoding:** `parse_endpoint` (dot ⇒ attached, else two ints) + serializer `push_endpoint` emits bare `x y` for free ends. Pinned: `free_endpoints_parse_to_cell_positions`, `endpoints_mix_attached_and_free`, `free_endpoint_with_non_numeric_coordinates_is_rejected`, `free_endpoint_missing_y_is_rejected`; the round-trip proptest strategy now generates both endpoint forms (byte-stable); all pre-existing goldens stayed byte-identical.
- **AC4 — free-end resolution:** `resolve_endpoint` matches on the sum type; a free end resolves to its stored cell with `side: None`. Pinned: `free_endpoint_resolves_to_its_cell_without_a_side`; new goldens `standalone-arrow.grid` (both ends free) and `mixed-endpoints.grid` (one attached, one free); the proptest suite now includes a standalone arrow so purity/determinism cover unattached endpoints.
- **AC5 — Arrow tool:** sidebar "↘ Arrow" button arms `Placing::Arrow`; press-drag-release creates the connector with per-end anchor snapping. Pinned: editor gesture tests `arrow_tool_draws_a_standalone_free_arrow` (both ends free at press/drop cells), `arrow_tool_snaps_its_ends_to_shape_anchors`, `arrow_tool_press_without_movement_draws_nothing`, `arrow_tool_refuses_a_zero_length_standalone_arrow`; tier-0b `arrow_tool_draws_a_standalone_arrow_via_the_sidebar` drives the real UI (sidebar click + canvas drag) and asserts the status-bar wording plus the model endpoints.
- **AC6 — endpoint drag:** `HitTarget::Endpoint { id, end }` hits the route's endpoint cells and visible tip glyph cells (below selected-shape anchors/resize handle, above bodies; declared bends keep priority); `AnchorExposure::All` exposes every anchor as a snap target while drawing/dragging. Pinned: canvas unit tests `hit_target_reports_endpoint_cells_above_bodies` (+ existing bend/segment/anchor priority tests updated to the new API); editor gesture tests `pressing_a_free_endpoint_without_moving_cycles_its_tip` (S-029 cycle preserved), `dragging_a_free_endpoint_moves_it_live`, `dropping_a_free_endpoint_on_an_anchor_attaches_it`, `dragging_an_attached_endpoint_off_its_anchor_detaches_it`; `PointerEvent::Up` now carries the release cell so free drops resolve.

New tests this story: 4 model + 4 parser unit + 1 pipeline unit + 2 golden scenarios (round-trip/proptest strategies extended) + 8 editor gesture + 1 tier-0b; Material goldens regenerated for the new sidebar button (diff confined to the sidebar band).
