---
title: S-013-gui-editing-gestures
description: full editing gesture set — move, resize (built-ins), anchor-to-anchor connect, select/delete; per-frame re-render
scope: gui-app
status: backlog
updated: 2026-10-22
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

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
