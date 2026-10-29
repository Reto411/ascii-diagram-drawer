---
title: S-018-gui-free-text
description: free text elements (FR10) — place, edit content, move like shapes; rendered through the same pipeline
scope: gui-app
status: done
updated: 2026-10-29
---

# S-018: gui-app — free text elements

## Why

FR10: independent text objects placed anywhere on the grid, editable in place; they render through the same pipeline (C2) so exports always match the canvas.

## Dependencies

- S-013 — selection/move gestures to reuse
- S-006 — text-object rendering in the place stage

## Acceptance criteria

1. **AC1 — Place + edit.** A "text" tool places a `TextInstance` at the clicked cell; double-click (or equivalent, recorded in Implementation notes) opens inline editing of its content; Enter/blur commits via the aggregate. Check: unit tests of the text gesture handler with fakes asserting `place_text`/content mutation.
2. **AC2 — Move like shapes.** Text objects are selectable and draggable with the same grid-snap rules as shapes (FR7). Check: unit test reusing the S-013 drag handler on a text object.
3. **AC3 — Render parity.** The edited content appears in `render` output byte-identically to what the canvas shows, including label-escaping edge cases from S-003 (quotes, long labels). Check: unit test comparing canvas grid bytes with `Exporter::to_text`.
4. **AC4 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80%; wasm check still exits 0.

## Implementation notes

The S-012…S-017 pattern: pure interaction logic in `gui-app` (headless-tested), thin egui glue in `app.rs`, model mutations only through the aggregate.

- **Model (`diagram-model`):** new `Diagram::set_text(&id, content)` — the aggregate operation for content edits; typed errors for unknown ids, non-text targets and empty content. `TextInstance::stamped_lines()` applies the C1 F4 quote fallback per line (lines containing any svgbob-alphabet glyph are wrapped in double quotes) — moved *into* the model so rendering and GUI hit-testing share one definition; new `glyphs.rs` holds the shared alphabet predicates (`is_line_glyph`, `is_arrowhead`, `is_svgbob_glyph`), which `render-pipeline` now delegates to (no behavior change).
- **Gesture machine (`gui-app/src/editor.rs`):** `placing: Option<ShapeInfo>` generalized to `Placing::{Shape(ShapeInfo) | Text}`; new `PointerEvent::DoubleClick` and outcomes `PlacedText` / `Editing` / `Edited`. Placement with the text tool places a placeholder object and opens inline editing in one gesture (the model rejects empty content, so a placeholder is required — it is replaced immediately). Double-click on a text body opens editing of its current content; shapes are not inline-editable in v1. While editing: a pointer press commits (blur semantics) and consumes the click; Escape cancels without mutation; an empty draft cancels instead of committing (the model rejects empties). The Move gesture now handles `Text` bodies identically to shapes (`HitTarget::Shape` → `Body`; grid-snap, grab-offset, origin clamp all unchanged).
- **Canvas (`gui-app/src/canvas.rs`):** hit-testing covers text via its stamped extent — new `text_hit_extent()` (per-line max width × line count of `stamped_lines`).
- **GUI glue (`gui-app/src/app.rs`):** sidebar gains a “T  Text” tool above the Shapes heading; while editing, a floating `egui::Area` hosts a multiline `TextEdit` bound to `EditorState::edit_draft`, positioned over the object’s cells. Plain Enter commits (checked *before* the field is shown so egui does not insert a newline first); Shift+Enter inserts line breaks; blur or pointer press commits; Escape cancels (via `on_escape`, which checks the edit first). While editing, Delete/Backspace are guarded out of `handle_key_frame` — they belong to the text field, not object deletion. The field requests keyboard focus exactly once when editing starts (it appears without a click).
- **Bug fix (pre-existing since S-012):** the canvas drew the trimmed grid text with the vertical content-origin offset but *not* the horizontal one — for any diagram whose content does not start at column 0, rendered cells sat left of the model/hit-test space (clicks on drawn objects hit empty cells). Both axes are now offset; the fix is pinned by `the_text_sample_fixture_loads` (box at x=3 renders with origin_x applied).
- **AC1 “or equivalent”:** double-click opens inline editing (the recorded choice); placement also opens it immediately, so content entry never needs a second round-trip.

## Verification

- [x] AC1 — **PASS** (tier 0, gui-testing skill: model state after user actions → unit tests, no screenshot). Headless gesture tests against a real `Diagram`: `editor::tests::the_text_tool_places_a_text_object_and_opens_editing` (click places `TextInstance` at the clicked cell + opens editing + consumes the tool), `committing_an_inline_edit_writes_the_content_to_the_model` (double-click → draft → Enter commits via `set_text`, multi-line content pinned), `committing_an_empty_inline_edit_cancels_without_mutation`, `escape_cancels_an_inline_edit_without_mutation`, `a_pointer_press_while_editing_commits_and_consumes_the_click` (blur semantics; no stray drag gesture); app-level `key_frames_during_inline_editing_protect_the_text_object` (Backspace while typing does not delete the object; Escape cancels). `cargo test -p gui-app` → **100 passed, 0 failed**.
- [x] AC2 — **PASS** (tier 0). `editor::tests::text_objects_are_movable_with_the_same_gesture_as_shapes` (press = select+grab, grid-aligned drag through the same S-013 handler) and `deleting_a_text_object_removes_it` (Delete cascades as for shapes).
- [x] AC3 — **PASS** (tier 0). `app::tests::canvas_frame_reflects_edited_text_bytes_exactly`: after a GUI-seam edit, the canvas frame carries the stamped bytes exactly — F4 quote wrap on the `|` line (`"a | b"`), plain lines verbatim, long line sets the extent (width 42 pinned). Shared-predicate tests: `objects.rs::stamped_lines_quote_only_lines_with_reserved_glyphs` (model) and the pipeline’s existing C1 table tests now delegate to the same `glyphs.rs` predicates. `app::tests::the_text_sample_fixture_loads` pins the new startup fixture (shape + single-line + multi-line text, content origin ≠ 0).
- [x] AC4 — **PASS.** `cargo fmt --check` clean; `cargo clippy --workspace --all-targets` zero warnings; `cargo test --workspace` all green (357 passed); `cargo tarpaulin --engine Llvm --workspace` → **82.89%** ≥ 80% (editor.rs 136/153, canvas.rs 91/94; app.rs 200/606 is the window-bound egui glue, as in S-012…S-017); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0 (its dead-code warnings are pre-existing — the app struct tree is unused until S-019’s wasm entry).
- [ ] Visual tiers (1/2) — **⚠️ deferred to image rebuild** (same environment gap as S-012…S-017: this container instance lacks winit’s X11 client libs, `libXcursor.so.1`; apt blocked by ADR 014 egress allow-list; Dockerfile already installs them). Post-rebuild: `sh .agents/skills/gui-testing/scripts/smoke.sh launch` (expect exit-124) and `sh .agents/skills/gui-testing/scripts/smoke.sh capture gui-app/tests/goldens/s018-text-sample.png 3 gui-app/tests/fixtures/text-sample.ad` (fixture added in this change: box + single-line + multi-line text at a non-zero content origin — proves free text renders end-to-end and pins the S-012 horizontal-origin bug fix). Manual checklist for a human with a display (`cargo run -p gui-app gui-app/tests/fixtures/text-sample.ad`): pick “T  Text”, click the canvas (placeholder appears + editor opens), type multi-line content (Shift+Enter), Enter commits, Esc on a fresh edit cancels, drag the text object, double-click it to re-edit. 
