---
title: S-012-gui-canvas-spike
description: eframe/egui app scaffold — WYSIWYG canvas rendering the resolved grid, sidebar with built-ins, click-to-place + drag with live re-render (ADR 008 first GUI milestone)
scope: gui-app
status: backlog
updated: 2026-10-22
---

# S-012: gui-app — eframe scaffold + WYSIWYG canvas spike

## Why

[ADR 008](../adr/008-gui-framework-choice.md) (proposed) sets its verification at exactly this milestone: grid canvas with drag + live re-render, built for desktop and browser. The canvas renders the *same* resolved grid as the export (§8.4.7 WYSIWYG decision), so the view can never diverge from the output. FR1/FR6/FR7 start here.

## Dependencies

- S-010 — GUI reaches the core only through `Services` (ADR 009); fakes for tests (T7)
- S-006 — resolved grid to draw

## Acceptance criteria

1. **AC1 — App builds natively.** `gui-app` is an eframe/egui binary; `cargo build -p gui-app` exits 0 on the host target; app owns `Services::native()` + a `Diagram` aggregate + transient view state (selection/hover only, per ADR 009). Check: build.
2. **AC2 — WYSIWYG canvas.** The canvas draws `Renderer::render(model)` in a monospace font (the exact bytes of the text output), with grid lines/selection overlays; pure helper functions for grid→screen mapping and cell hit-testing are unit-testable without a window. Check: unit tests on mapping/hit-test with fixture grids.
3. **AC3 — Sidebar + place.** Template sidebar lists built-in shapes from `ShapeCatalog::list`; clicking a template then clicking the canvas places a shape at the grid-snapped cell (FR1, FR7); document order = z-order preserved. Check: interaction-logic unit tests against fake `Services` (T7) asserting the aggregate mutation sequence.
4. **AC4 — Drag with live re-render.** Dragging a selected shape moves it (grid-snapped) and the canvas re-renders every frame from the mutated model (RS1/RS3; no layout pass). Check: unit test of the drag gesture handler (pointer down/move/up events → `move_object` calls, snap to cells).
5. **AC5 — Wasm compiles.** `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0 (full web bundle in S-019). Check: compile check.
6. **AC6 — ADR 008 flipped to accepted** (first GUI milestone met); manual smoke checklist (open app, place, drag) written into the story's Verification for a human with a display; overview tracker + C2 doc WYSIWYG note updated.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
