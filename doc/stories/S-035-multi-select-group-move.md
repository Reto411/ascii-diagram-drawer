---
title: S-035-multi-select-group-move
description: CR-017 — Ctrl/Shift+click multi-select + group move (FR20); selection set in EditorState, group drag, bulk delete
scope: gui-app
status: done
updated: 2026-10-31
---

# S-035: gui-app — Ctrl/Shift+click multi-select + group move (FR20)

## Why

FR20: select several shapes with Ctrl/Shift+click and move the whole selection by dragging any member (same delta for every member); Delete removes every selected object with its connectors; Escape clears the selection. Standard editor behavior, needed for repositioning clusters in daily UML work.

## Dependencies

- none beyond done stories (S-013)

## Acceptance criteria

Written from [CR-017](../crs/CR-017-multi-select-group-move.md) "Requested change" + §14 FR20 (already in place since the 2026-10-31 bulk acceptance).

- **AC1 (modifier selection)** — Ctrl or Shift held while clicking a shape body adds it to the current selection without clearing existing members; modifier-clicking an already-selected member removes it again (toggle); a plain click replaces the set with just that shape. A modifier-click on empty space leaves the selection untouched; a plain click on empty space still clears it.
- **AC2 (group move)** — dragging any selected member moves every movable selected member (shapes and free text) by the same delta, each preserving its grab offset and clamping at the grid origin. Connectors in the selection are not moved themselves; their attached endpoints follow their bound shapes as today.
- **AC3 (selection outline)** — the canvas draws the selection outline around every selected member's bounds (shapes and free text), not only one object's.
- **AC4 (delete + escape)** — Delete/Backspace removes every selected object together with any connector referencing a removed object; Escape clears the whole selection set. No selection → no-op, as today.
- **AC5 (anchors + resize)** — in selection mode the anchor dots are exposed for every selected shape; the resize handle is offered only when exactly one shape is selected (v1 keeps resize single-selection per CR-017's conflict check against S-024).

## Implementation notes

- `EditorState.selection` becomes a `BTreeSet<ObjectId>` (deterministic order); `ObjectId` gains an `Ord` derive (lexicographic over the token — membership/ordering only, never user-visible).
- `PointerEvent::Down` carries the additive modifier state; the egui glue reads Ctrl/Shift from `ctx.input()` at the press frame. The app seam gains `handle_pointer_frame_additive(held, hit, additive)` (the real path); the old `handle_pointer_frame(held, hit)` survives as a `#[cfg(test)]` convenience wrapper with modifiers off, so existing tests are untouched.
- `Gesture::Move` becomes a group move: one entry per movable member with its grab offset; each member follows the pointer exactly like today's single move (same delta, clamped at origin). A one-member group still reports `EditOutcome::Moved` for status-bar continuity.
- Toggle-off / modifier-add report the new set size via `EditOutcome::SelectionChanged`; a bulk delete reports `EditOutcome::BulkDeleted`.
- Canvas: `hit_target`/`overlays` take the selection set; anchor exposure covers all selected shapes; the resize handle and its marker require exactly one selected shape (AC5).

## Verification

- **AC1 (modifier selection)** — ✅ tier 0: `editor::tests::ctrl_click_on_a_second_shape_extends_the_selection` (plain click → `Selected`, Ctrl+click on a second shape → `SelectionChanged { count: 2 }`, both ids in the set), `ctrl_click_on_a_selected_member_toggles_it_off` (Ctrl+click removes a member, set size back to 1), `additive_press_on_empty_space_keeps_the_selection` (modifier press on empty space → `Nothing`, selection kept); plain-click-on-empty-space still deselects (`clicking_an_empty_cell_deselects`, pre-existing). Tier 0b (real egui events incl. `ModifiersChanged(CTRL)`): `gui_tests::ctrl_click_builds_a_group_and_drag_moves_it` asserts `app.selection().len() == 2` after a real Ctrl+click and the status bar text "2 objects selected"; `ctrl_click_on_empty_space_keeps_the_selection` asserts the set survives a modifier click on empty space.
- **AC2 (group move)** — ✅ tier 0: `drag_after_ctrl_click_moves_the_whole_group` (both members get the exact same (+6,+3) delta, 20-col spacing preserved, `GroupMoved { count: 2 }`), `plain_click_on_a_selected_member_moves_the_group` (FR17 wording: a PLAIN drag on any selected member of an existing multi-selection keeps the set and moves it all), `plain_click_on_the_only_selected_object_moves_only_it` (S-013 base case unchanged — one-member group reports `Moved` as before), `group_move_skips_connectors_in_the_selection` (connector in the set is not moved itself; only the two boxes shift, connector survives). Tier 0b: `ctrl_click_builds_a_group_and_drag_moves_it` drags a real pointer across the canvas and asserts both model positions advanced by the same (+8,+2) delta.
- **AC3 (selection outline)** — ✅ tier 0b + tier 2: `ctrl_click_builds_a_group_and_drag_moves_it` counts exactly two SELECTION-colored stroked rects in the frame's FullOutput. Pixel scan of dump shot 14 (`GUI_DUMP=1 cargo test -p gui-app visual_dump` → `/tmp/gui-dump/14-multi-select.png`, added as durable scenario 14): two separate selection outlines — box A x 212..289, box B x 321..398, both rows 112..193 (78×81 px each), 1244 SELECTION pixels total.
- **AC4 (delete + escape)** — ✅ tier 0: `delete_with_a_multi_selection_removes_everything` (`BulkDeleted { count: 2, removed: 3 }` — two boxes plus their cascaded connector, model empty, selection cleared); `escape_clears_the_whole_selection` (two-member set → `Deselected`, set empty). No-selection no-ops stay pinned by the pre-existing `delete_without_a_selection_is_a_noop` / `escape_cancels_gestures_placement_and_selection`.
- **AC5 (anchors + resize)** — ✅ tier 0: `canvas::tests::hit_target_multi_selection_exposes_all_anchors_and_no_resize` (both members' e anchors hit-test as `Anchor`, the bottom-right cell falls back to `Body`), `overlays_multi_selection` (single selection → 4 anchor dots + resize handle; multi → 8 anchor dots, no resize handle). Tier 2 pixel scan of shot 14: exactly 8 ANCHOR-colored clusters — 4 per box (top-center, left/right mid, bottom-center) — and zero anchor-color pixels in either box's bottom-right corner, i.e. no resize marker drawn.
- **Gates** — `cargo fmt --all` clean; `cargo clippy --workspace --all-targets` → 0 warnings/errors; `cargo test --workspace` → 627 passed / 0 failed (incl. 13 new S-035 tests across tiers 0/0b); `cargo tarpaulin --engine Llvm --workspace` → 93.61% (target ≥ 80%, no drop). 
