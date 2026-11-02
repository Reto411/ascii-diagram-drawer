---
title: S-041 - arrow-object-behavior-and-line-handles
description: CR-020 — connectors behave as normal objects (body drag moves the whole arrow), visible draggable endpoint/bend handles, double-click bend insertion, in-canvas tip dropdown per end
scope: gui-app
status: done
updated: 2026-11-02
---

# S-041: Arrow object behavior and Visio-style line handles

## Why

[CR-020](../crs/CR-020-arrow-object-behavior-and-line-handles.md) (accepted 2026-11-01, FR9): arrows are the least intuitive objects on the canvas — they cannot be moved like shapes, their endpoints give no visible affordance, adding bends is a hidden gesture, and tip selection is an undocumented press-cycle. They shall behave as normal objects with Visio-style line handles: drag body moves the whole arrow; selected arrows show visible square handles at both endpoints and every bend, all draggable; double-click on a segment inserts a bend; pressing a selected endpoint opens an in-canvas tip dropdown. Both connector kinds (shape-to-shape and standalone).

## Dependencies

- S-028 (done) — manual bends: body-drag gesture reworked to whole-arrow move, double-click insertion added
- S-029 (done) — per-end tips: press-to-cycle replaced by the in-canvas dropdown trigger
- S-031 (done) — free endpoints: endpoint drag/detach/reattach semantics extended with visible handles

## Acceptance criteria

Written from CR-020's requested change (items 1–5), 2026-11-02.

- **AC1 — Move like an object.** Pressing a connector's body selects it; dragging the body moves the *whole* connector — both endpoints and every declared bend translate by one delta. A bound end detaches on the first movement (becoming free at its anchor cell) and, on release, re-attaches only if dropped exactly on an anchor; otherwise it stays free. A press without movement only selects (no model change). The old "drag from body adds a bend" gesture is gone. Both connector kinds (shape-to-shape and standalone).
- **AC2 — Visio line handles.** While a connector is selected (selection mode), the canvas shows visible square handles at both endpoints and at every declared bend, in addition to today's bend handles; endpoint handles sit on the resolved endpoint cell (anchor cell for bound ends) and start the existing endpoint-drag gesture; bend handles keep the existing bend-move gesture with its axis-aligned-segment constraint and collinear-drop removal.
- **AC3 — Double-click inserts a bend.** Double-clicking a connector's body segment inserts a new bend (waypoint) at the clicked cell, selects the connector, and reports `BendAdded`. Double-clicking an existing bend, an endpoint, or empty space does not insert anything.
- **AC4 — In-canvas tip dropdown.** Pressing onto a selected endpoint without moving opens an in-canvas dropdown listing the four tip styles (none / hollow arrow / empty rhombus / filled rhombus) for that end; picking one applies it via `set_tip` and closes the menu. This replaces press-to-cycle entirely (`cycle_tip` removed). Right-click/Escape close the menu without change (right-click keeps its cancel role, S-013); any other canvas press closes it first.
- **AC5 — Both kinds.** AC1–AC4 hold for shape-to-shape connectors and standalone arrows alike.

Feature bar: `ui_decomposition_*` tests draw representative diagrams purely through the real UI — a connected pair reshaped by body-drag, double-click bend insertion, handle drag and dropdown tip selection (shape-to-shape), plus a standalone-arrow variant.

## Implementation notes

Filled in while working the story, 2026-11-02. All changes in `gui-app` (the editor/canvas/app seam; no model or ADR changes — CR-020 accepted without ADR impact, [ADR 013](../adr/013-connector-routing.md) untouched).

- **AC1 — whole-arrow body move** (`editor.rs`): new `Gesture::ConnectorMove { id, pressed, from_cell, to_cell, bends, moved }`. Pressing a body cell selects and snapshots the press-time endpoints + bends; every move translates *both* ends and all bends by the pointer delta from that snapshot — a bound end detaches in place (free at its anchor cell) on the first displacement. Release reports `ConnectorMoved`; the old S-028 "drag from body adds a bend" gesture is gone. Re-attach on release lives in `app.rs::reattach_moved_ends`: after a move, each free end dropped exactly on an anchor is re-attached, and its message wins the status bar.
- **AC2 — Visio line handles** (`canvas.rs`): the selection overlay now draws square handles at both endpoints and every bend of the selected connector (bend handles keep their gesture). `hit_target` exposes endpoint cells above bodies — the resolved endpoint cell plus, when the tip is non-bare and the route is long enough, the adjacent glyph cell (the visible arrowhead is pressable; same convention as the S-029 anchor markers) — and body segments carry `from_cell`/`to_cell` for the move gesture. Bend-move keeps its axis-aligned-segment constraint and collinear-drop removal.
- **AC3 — double-click bend insertion** (`editor.rs`): `on_double_click` routes `HitTarget::ConnectorBody` to `diagram.add_bend(id, segment, cell)` using the hit's segment index; the press of the pair only selected. Any other target keeps S-018 behavior (text/shape) or is a no-op.
- **AC4 — in-canvas tip dropdown** (`editor.rs` + `app.rs`): press-without-move on an endpoint now opens `tip_dropdown: Option<(ObjectId, ConnectorEnd)>` instead of cycling (`cycle_tip` removed from the codebase). The app draws it as an egui area anchored at the endpoint ("Tip — obj-N (source|target)" + four rows); picking a row calls `apply_tip` → `set_tip` for that end only and closes. The drawn rect is stored (`tip_menu_rect`) so a primary press inside it skips canvas pointer handling (the row click reaches the menu, whose `clicked()` fires on release); any other press closes the menu first and then acts; right-click/Escape close without change via `on_escape` (right-click keeps its S-013 cancel role).
- **AC5 — both kinds**: every gesture above is endpoint-model-driven, so shape-to-shape and standalone arrows share it; standalone coverage comes from the arrow-tool tests plus the decomposition's standalone variant.
- **Feature bar** (`gui_tests.rs`): `ui_decomposition_arrow_handles` draws the representative diagram purely through the UI — sidebar → place two boxes → connect — then body-drag (both ends detach), double-click bend insertion, bend-handle drag, endpoint-handle drag with re-attach onto a shape anchor, and the tip dropdown; a standalone arrow drawn with the sidebar tool gets the same double-click + dropdown gestures. New harness helpers: `settle(n)` (idle frames to clear egui's 0.3 s double-click window — without it the pair's second click is classified as a *triple* and `button_double_clicked` stays false) and `right_click_at`.

## Verification

All gates, 2026-11-02: `cargo fmt --all` clean; `cargo clippy --workspace --all-targets` → 0 warnings/errors; `cargo test --workspace` → **700 passed, 0 failed**; `cargo tarpaulin --engine Llvm --workspace` → **93.42%** (target ≥ 80%); `mkdocs build --strict` → exit 0.

- [x] AC1 — `editor::tests::pressing_a_connector_body_and_dragging_moves_the_whole_arrow` (press-only selects; first displacement translates ends + bend, both ends detach; off-anchor drop stays free) + `app::tests::reattach_moved_ends_reattaches_only_ends_on_anchors` + feature-bar body-drag step in `ui_decomposition_arrow_handles` — all pass.
- [x] AC2 — `canvas::tests::overlays_show_endpoint_handles_for_the_selected_connector` (squares at both endpoints + every bend) + `canvas::tests::hit_target_reports_endpoint_cells_above_bodies`; bend gesture unchanged: `editor::tests::pressing_a_bend_selects_the_connector_and_moves_it_live`, `dropping_a_moved_bend_on_the_neighbor_line_removes_it`, `bend_dragging_is_free_with_exact_axis_alignment` + feature-bar handle-drag steps (bend drag; endpoint drag re-attaching onto B's south anchor) — all pass.
- [x] AC3 — `editor::tests::double_clicking_a_connector_body_inserts_a_bend_there` (`BendAdded`, connector selected) + `double_clicking_a_bend_endpoint_or_empty_space_inserts_nothing` (negatives) + feature-bar double-click steps on both connector kinds — all pass.
- [x] AC4 — `editor::tests::pressing_an_endpoint_without_moving_opens_the_tip_dropdown` (release without move opens; no cycle), `applying_a_tip_from_the_dropdown_changes_only_that_end`, `escape_closes_the_tip_dropdown_without_mutation`; GUI: `gui_tests::pressing_the_connector_tip_opens_a_dropdown_that_applies_styles` (four rows, glyph follows, reopens) + `the_tip_dropdown_closes_on_right_click_and_outside_press` (right-click cancels; outside press closes first then deselects); `cycle_tip` no longer exists in the tree — all pass.
- [x] AC5 — `editor::tests::arrow_tool_draws_a_standalone_free_arrow`, `arrow_tool_refuses_a_zero_length_standalone_arrow`; GUI: `gui_tests::arrow_tool_draws_a_standalone_arrow_via_the_sidebar` + the standalone variant inside `ui_decomposition_arrow_handles` (double-click bend + endpoint dropdown) — all pass.
