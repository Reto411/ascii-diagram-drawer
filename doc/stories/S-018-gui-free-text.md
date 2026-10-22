---
title: S-018-gui-free-text
description: free text elements (FR10) — place, edit content, move like shapes; rendered through the same pipeline
scope: gui-app
status: backlog
updated: 2026-10-22
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

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
