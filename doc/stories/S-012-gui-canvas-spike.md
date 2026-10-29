---
title: S-012-gui-canvas-spike
description: eframe/egui app scaffold — WYSIWYG canvas rendering the resolved grid, sidebar with built-ins, click-to-place + drag with live re-render (ADR 008 first GUI milestone)
scope: gui-app
status: done
updated: 2026-10-29
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

- **Canvas = model space, not grid space.** The pipeline trims the resolved grid to the occupied extent (`ResolvedGrid` carries no origin), so a shape at model cell (3,4) renders as row/column 0 of the output. `gui-app` therefore keeps the viewport mapping *model space* → pixels with a fixed margin, draws the exact text bytes offset by the diagram's content origin (`DrawingApp::content_origin()`), and maps pointer positions to unbounded model cells (`Viewport::screen_to_model_cell`). Placing/dragging in model coordinates stays stable even when the trim origin shifts.
- **egui 0.37 API notes** (the crate moved to a scene-based API in 0.36+): `eframe::App::ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)`; `NativeOptions`/`run_native` are `#[cfg(not(wasm))]` (wasm entry = `WebRunner`, S-019); no `Context::set_animations_enabled` anymore (egui 0.37 has no animation toggle — the UI is static, so settled frames are stable for goldens). Glyph advance is measured per frame via `painter.layout("MMMMMMMMMM", …)` so text lands cell-exact under any font metrics.
- **Test seam:** `DrawingApp::handle_pointer_frame(held, cell) -> EditOutcome` converts raw egui pointer edges (press/release) into the pure gesture machine in `editor.rs`; right-click cancels placement. All interaction logic is tested headless through this seam with fake `Services` (T7).
- **Environment gap (runtime only):** the running devcontainer image predates the winit X11 client libraries — `libXcursor.so.1`, `libXi.so.6`, `libxkbcommon.so.0` are absent and apt is blocked by the egress allow-list (ADR 014), so tier-1 launch-alive cannot run *in this container instance*. The Dockerfile now installs them (`libxcursor1 libxi6 libxkbcommon0` in the xvfb layer); after the next image rebuild, `sh .agents/skills/gui-testing/scripts/smoke.sh launch` is expected to pass without code changes.

## Verification

- [x] AC1 — `cargo build -p gui-app` exits 0 with **zero warnings** (2026-10-29). `gui-app` is an eframe/egui 0.37 binary; `DrawingApp` owns `Services::native()` + a `Diagram` aggregate; selection/placement are transient view state in `editor.rs` (never written to the model) — ADR 009 respected.
- [x] AC2 — canvas draws the exact `Renderer::render` → `to_text` bytes in monospace at the measured glyph advance, with grid lines + selection overlay + placement ghost (`app.rs::draw_canvas`). Pure mapping/hit-test helpers unit-tested without a window: `canvas.rs` 16/16 lines covered (round-trip every cell, edge inclusivity, unbounded model space, margin rejection); `canvas_frame_is_the_exact_rendered_bytes_after_placement` pins the WYSIWYG bytes + content-origin offset.
- [x] AC3 — sidebar lists `ShapeCatalog::list()` built-ins (`box`, `ellipse`); click template → click canvas places at the snapped cell (single-shot placement, new shape selected), topmost-on-overlap hit test, document order = z-order — all asserted in `editor.rs` headless tests against fake `Services` (`clicking_a_template_then_the_canvas_places_at_the_clicked_cell`, `clicking_a_shape_selects_the_topmost_on_overlap`, `document_order_is_z_order_for_multiple_placements`).
- [x] AC4 — drag = select + move in one gesture: press inside a shape records the grab offset, every subsequent move calls `move_object` grid-snapped with origin clamping (`dragging_clamps_at_the_grid_origin`, `a_single_gesture_both_selects_and_drags_the_shape`); the canvas re-renders from the mutated model every frame (immediate mode — no layout pass), pinned by `pointer_frames_drive_place_select_and_drag` through the `handle_pointer_frame` seam.
- [x] AC5 — `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0. Wasm entry is a documented no-op stub for now (all app modules compile + type-check for wasm); the real `WebRunner` glue lands in S-019.
- [x] AC6 — ADR 008 flipped to **accepted v3** (no fallback to Slint needed; API churn manageable). Overview tracker updated; C2 concept doc WYSIWYG note updated with the implementation location. Manual smoke checklist for a human with a display:
  1. `cargo run -p gui-app` → window opens (~1024×768, dark theme), empty canvas with status bar.
  2. Click **box** in the sidebar (ghost outline follows the cursor) → click a grid cell → box appears there, selected (blue outline), status bar shows `Placed box at (col, row)`.
  3. Press inside the box and drag → it moves cell-snapped while re-rendering live; release → status bar shows the new position.
  4. Click empty space → selection clears. Right-click during placement mode cancels it.
  5. Headless equivalent (once the image is rebuilt with the X11 libs): `sh .agents/skills/gui-testing/scripts/smoke.sh launch` (expect exit-124 PASS) and `… capture gui-app/tests/goldens/empty-canvas.png 3` + `… capture gui-app/tests/goldens/one-box.png 3 -- gui-app/tests/fixtures/one-box.ad` — ⚠️ **deferred to human/rebuild**: the current container instance lacks `libXcursor`/`libXi` (apt blocked by ADR 014 egress allow-list); Dockerfile fixed in this change.
- Coverage: workspace `cargo tarpaulin --engine Llvm --workspace` = **92.30%** ≥ 80% (gui-app per-crate is lower because the egui draw glue + bootstrap are window-bound; logic modules canvas.rs/editor.rs are 100%/93%). Workspace fmt/clippy(0 warnings)/test green — 20 new gui-app tests, all suites pass. 
