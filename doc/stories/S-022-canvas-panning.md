---
title: S-022-canvas-panning
description: CR-014 — canvas panning for diagram navigation (FR17): middle-drag or scroll shifts the view; pure view state, gestures keep working at any offset
scope: gui-app
status: done
updated: 2026-10-31
---

# S-022: gui-app — canvas panning for diagram navigation (FR17)

## Why

FR17 (High): at scale 1× a diagram larger than the window is clipped and its off-screen parts are unreachable — no selection, no editing. Panning shifts which part of model space is displayed; it never changes the model or its rendering (NFR3), and every existing canvas gesture must keep working at any pan offset.

## Dependencies

- none beyond done stories (S-012/S-013)

## Acceptance criteria

Written from [CR-014](../crs/CR-014-canvas-panning.md)'s "Requested change" + the §14 FR17 wording.

- **AC1 — pan gesture:** dragging with the middle mouse button held on the canvas pans the view; the content follows the pointer by exactly the drag delta. Panning never modifies the diagram model (NFR3) — it is view state only (`DrawingApp.pan`, pixels).
- **AC2 — scroll also pans:** scrolling over the canvas area pans the view by egui's scroll delta (content moves in the direction of the delta); scrolling over the sidebar or top bar does not pan.
- **AC3 — bounds & reset:** the pan is clamped at the model origin (nothing exists before cell 0, so no panning left/up past it). Opening a document resets the pan to zero.
- **AC4 — gestures keep working at any offset:** hit-testing, selection and overlays derive from the same panned `Viewport`, so a shape can be selected/moved by clicking where it is *now* after panning; existing gesture tests (place/select/move/connect/resize) still pass unchanged at pan = 0.

## Implementation notes

- `gui-app/src/canvas.rs` — `Viewport::new(margin, …)` replaced by `Viewport::with_origin(origin_x, origin_y, cell_w, cell_h, scale)`; the mapping math is unchanged (a margin viewport is `with_origin(m, m, …)`). Callers clamp the pan.
- `gui-app/src/app.rs` — `DrawingApp.pan: Vec2` + a `middle_held` grab latch (the press frame only latches, so a synthetic move+press batch cannot jump the view). Middle-drag and scroll both apply `pan -= delta` (egui's delta says how the *content* should move); scroll is gated to the canvas rect via `pointer.interact_pos()`; pan clamped at 0. While a primary-button gesture runs, middle-drag does not pan (the drag belongs to the editor).
- The viewport origin becomes `MARGIN_PX - pan`; content rows, grid paper lines, overlays and hit-testing all derive from that one viewport — so panning shifts everything coherently.
- `fileio::open` success path resets `pan` to zero (with `editor.reset()`).

## Verification

Tier 0 (headless unit) + tier 0b (full-app egui frame pumping, `gui_tests.rs`):

- `canvas::tests::panned_viewport_maps_from_a_negative_origin` — a panned origin maps cells and hit-tests back correctly, including at scale ×2.
- `gui_tests::middle_drag_pans_the_canvas_and_hit_testing_follows` — middle-drag (60, 30) left/up: `pan == (60, 30)`; the drawn box's top-border row moved by exactly (−60, −30); clicking where the body is *now* selects it (SELECTION stroke present).
- `gui_tests::scroll_pans_the_canvas_but_not_over_the_sidebar` — 15 × −4 px point deltas over the canvas → `pan == (0, 60)` and the content row moved up 60 px; the same scroll over the sidebar leaves the pan untouched.
- `gui_tests::pan_is_clamped_at_the_model_origin` — a right/down middle-drag from zero stays at `pan == 0`.
- `gui_tests::opening_a_document_resets_the_pan` — after panning to (100, 50), opening `tests/fixtures/one-box.ad` resets the pan to zero.
- Full regression: `cargo test --workspace` green (148 gui-app tests); `cargo clippy --workspace --all-targets` clean; workspace coverage via `cargo tarpaulin --engine Llvm --workspace` ≥ 80% (see commit).
- No visual change at rest: pan = 0 renders byte-identically to before (origin math collapses to the old margin-based viewport); no committed goldens exist (visual dump tier is opt-in, `GUI_DUMP=1`).
