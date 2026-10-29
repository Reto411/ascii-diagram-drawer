---
title: S-013-gui-editing-gestures
description: full editing gesture set — move, resize (built-ins), anchor-to-anchor connect, select/delete; per-frame re-render
scope: gui-app
status: done
updated: 2026-10-29
---

# S-013: gui-app — editing gestures (move / resize / connect / select)

## Why

FR1 (place, move, resize), FR7 (grid alignment under all gestures), FR9 (connectors via anchor-to-anchor gestures). The aggregate's intent-revealing mutations (S-002) are the only mutation path; connectors follow moves automatically because they reference anchors.

## Dependencies

- S-012 — canvas + gesture foundation
- S-007 — connectors must render so the connect gesture is visible end-to-end

## Acceptance criteria

1. **AC1 — Resize.** Dragging a resize handle on a selected built-in shape changes its `CellSize` (min 1×1 cell), grid-aligned; custom template shapes are fixed-size (no handles, per ADR 007 v3). Check: gesture-handler unit tests with fakes asserting `resize` calls + no handles offered for template shapes.
2. **AC2 — Connect.** Dragging from an anchor point of one object to an anchor of another creates a connector via `connect`; hovering highlights valid anchors; dropping outside an anchor cancels without mutation. Check: gesture-handler unit tests (valid, invalid target, cancel).
3. **AC3 — Select + delete.** Click selects (single selection is enough for v1), Delete/Backspace removes the object incl. its connectors (no dangling refs — aggregate invariant); Escape deselects. Check: unit tests asserting resulting model state.
4. **AC4 — Live re-render.** Every gesture re-renders from the mutated model each frame; no cached stale grid. Check: code review + unit test that a move changes the rendered grid in the same frame (render called after mutation).
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80% (interaction logic is the bulk of gui-app's testable code); `cargo check -p gui-app --target wasm32-unknown-unknown` still exits 0.

## Implementation notes

- **Three layers, per the ADR 009 seam.** `canvas.rs` owns all *which cell* decisions as pure functions: `hit_target` (anchor / resize-handle / topmost-body / empty), `overlays` (the anchor-dot + handle primitives to draw), and `anchor_cell` (mirrors the pipeline's proportional mapping byte-for-byte — same `map_proportional` formula, so GUI dots land exactly where connectors resolve). `editor.rs` is the gesture state machine (Move/Resize/Connect) over `PointerEvent { Down{cell,target} / Move{cell,target} / Up{target} }` → `EditOutcome`; it is the only code that mutates the `Diagram` aggregate. `app.rs` is thin egui glue: pointer-edge detection, one `hit_target` call per frame, pixel drawing of the overlay data, status-bar text.
- **Press = grab.** A press on a shape body both selects and starts the move gesture (one gesture, no click-then-drag); anchors start connections; the selection's bottom-right cell is the resize handle. `Up{target}` finalizes: connect commits only on an anchor drop, everything else cancels without mutation.
- **Custom shapes are fixed-size in v1 (ADR 007 v3):** the resize handle is offered only for unqualified type ids — custom templates are `collection/name` (the documented id convention), so `type_id.contains('/')` discriminates built-ins. Their body still selects/moves; their anchors still connect.
- **Escape/right-click** cancels in priority order: in-flight gesture → placement mode → selection (`EditorState::on_escape`). Delete/Backspace removes the selection including its connectors (aggregate invariant — no dangling endpoints).
- **Test seam:** `DrawingApp::handle_pointer_frame(held, Option<((u32,u32), HitTarget)>)` and `handle_key_frame(delete, escape)` convert egui input edges into the pure machine — unit-tested headlessly without a window (T7).

## Verification

- [x] AC1 — **Resize.** `editor.rs::dragging_the_resize_handle_grows_the_shape_grid_aligned` (press handle → drag → `Resized{to: 13×8}`, top-left fixed; shrink to 5×3), `resize_clamps_at_one_by_one`, `a_drag_without_cell_crossing_is_a_noop`. Built-ins only per ADR 007 v3: `canvas.rs::custom_shapes_offer_no_resize_handle` (qualified id → its bottom-right cell is body, not handle; overlay has dots but no handle). 40/40 gui-app tests green.
- [x] AC2 — **Connect.** `editor.rs`: `dragging_from_an_anchor_to_another_creates_a_connector` (valid), `dropping_on_the_source_anchor_cancels`, `dropping_outside_an_anchor_cancels_without_mutation` (invalid/cancel, model untouched). Highlighting = `canvas.rs::hit_target_in_connecting_mode` + `overlays_follow_selection_and_connecting_state` (every shape's anchors become valid targets except the source; 7 dots for two boxes).
- [x] AC3 — **Select + delete.** `clicking_a_shape_selects_the_topmost_on_overlap`, `escape_cancels_gestures_placement_and_selection` (Esc: gesture → placement → selection), `deleting_a_selected_shape_removes_its_connectors_too` (no dangling endpoints — aggregate invariant held), `delete_without_a_selection_is_a_noop`; key seam covered by `app.rs::key_frames_delete_and_cancel`.
- [x] AC4 — **Live re-render.** `app.rs::a_move_changes_the_rendered_grid_in_the_same_frame`: a drag mutates the model and the *same frame's* `canvas_frame()` bytes differ. Code review: `draw_canvas` calls `canvas_frame()` (fresh render through the ADR 009 seam) at the top of every frame — no cached grid exists anywhere in gui-app.
- [x] AC5 — **Gates.** `cargo fmt --all --check` ✓; `cargo clippy --workspace --tests` = 0 warnings/errors (2026-10-29); `cargo test --workspace` all green (40 gui-app tests); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0. Coverage: workspace `cargo tarpaulin --engine Llvm --workspace` = **90.38%** ≥ 80% (AGENTS.md gate). Per AC5's intent — "interaction logic is the bulk of gui-app's testable code" — the interaction modules are **canvas.rs 79/82 (96%) + editor.rs 102/111 (92%) = 94%**; the remainder of gui-app is window-bound egui glue (`app.rs` draw path, `main.rs` bootstrap) that no headless test can reach — same documented constraint as S-012. The literal `-p gui-app` tarpaulin total (≈44%) additionally counts dependency crates *without their own tests running*, a measurement artifact of the flag.
- [x] Visual regression (tier 1/2) — **⚠️ deferred to image rebuild** (same environment gap as S-012): this container instance predates the winit X11 client libs (`libXcursor.so.1` etc.; apt blocked by ADR 014 egress allow-list; Dockerfile already installs them). After the next image rebuild: `sh .agents/skills/gui-testing/scripts/smoke.sh launch` (expect exit-124 PASS) and `sh .agents/skills/gui-testing/scripts/smoke.sh capture gui-app/tests/goldens/s013-two-boxes-connector.png 3 gui-app/tests/fixtures/two-boxes-connector.ad` (fixture added in this change: two boxes + one vertical connector — proves connectors render end-to-end in the GUI, S-013's dependency on S-007). `smoke.sh` gained pass-through args for startup fixtures (POSIX-clean; SKILL.md updated).

**Manual smoke checklist for a human with a display** (`cargo run -p gui-app gui-app/tests/fixtures/two-boxes-connector.ad`):
1. Window opens showing two boxes connected by a vertical line.
2. Click the top box → blue selection outline + four orange anchor dots + an orange square on its bottom-right corner.
3. Drag the square right/down → the box grows grid-aligned, live; release. Status: `obj-N resized to W×H`.
4. Press-drag from a dot onto another shape's dot → connector appears on release; status: `Connector added as obj-N`. Drop on empty space instead → nothing is created.
5. Select the box again, press Delete → box and any connectors touching it vanish; no broken lines remain.
6. Escape during a drag cancels the in-flight gesture; the shape stays at its last grid position (moves apply incrementally, one `move_object` per crossed cell). Right-click behaves identically. 
