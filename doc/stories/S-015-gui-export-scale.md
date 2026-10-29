---
title: S-015-gui-export-scale
description: export menu — SVG save/download, PNG via worker with "exporting" state (RS4) + scale control (FR8)
scope: gui-app
status: done
updated: 2026-10-29
---

# S-015: gui-app — SVG/PNG export + scale control

## Why

FR5 (SVG/PNG export from the same resolved grid) and FR8 (scale control scales the *rendering output*, not display zoom); RS4 (export may take time — show "exporting", never freeze the UI) is satisfied via the S-011 worker API.

## Dependencies

- S-012 — app shell + canvas
- S-011 — PNG worker API with pollable state
- S-008, S-009 — text/SVG/PNG exporters behind `Services`

## Acceptance criteria

1. **AC1 — SVG export.** "Export SVG" produces the exact `Exporter::to_svg` bytes and saves them (native: file; web: download); a failure notice shows on error. Check: unit test of the export flow with fake services asserting the sink receives the exporter's exact bytes.
2. **AC2 — PNG export, non-blocking.** "Export PNG" submits to the worker API; while pending the UI shows an "exporting…" state and stays responsive (no blocking call on the UI thread); on completion the PNG is saved/downloaded or an error notice appears. Check: unit test with a fake slow exporter asserting Pending → Done transitions drive the UI state machine; code review confirms no rasterization call on the frame path.
3. **AC3 — Scale control.** A scale control (top-right area per FR8) changes the model-level scale via the aggregate, re-renders the canvas, and affects all exports (text/SVG/PNG) identically; default 1. Check: unit test asserting scale change → `render`/`expand` output differs and export bytes follow.
4. **AC4 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80%; wasm check still exits 0.

## Implementation notes

- **New module `gui-app/src/exportio.rs`** — the pure export flows (S-012…S-017 pattern: plain functions over the seam services, unit-tested headlessly against fakes): `svg_bytes()` (render → `to_svg`, exact bytes), `export_output_path()` (sibling-path rule shared with S-014's `.txt`), `PngExport` (holds the worker's `PngJob`; `start`/`start_grid` submit, `tick()` polls non-blocking and consumes the job on a terminal state → `PngTick::{Done, Failed}`), `write_bytes_file()` (native `std::fs`; wasm typed error until S-019 — same sanctioned exception as S-014). 100% line coverage.
- **Seam wiring (ADR 009 v7):** `ui-api` re-exports `ResolvedGrid`/`RenderError` (already in its trait signatures; keeps `gui-app` inside the documented dependency edge), and the composition root exposes `Services.png_worker: Arc<PngWorker>` via new `PngWorker::for_target()` (threaded on native, inline on wasm32 — ADR 012). `PngJob::from_reply` became public as the test seam for channel-driven jobs.
- **App glue (`app.rs`):** toolbar gains “Export SVG…” / “Export PNG…” (PNG button disabled while a job is in flight) → inline path prompts (new `PathPrompt::{ExportSvg, ExportPng}`, prefilled with the document's sibling path); `tick_png_export()` runs once per frame at the top of `ui()` — nothing on the frame path blocks (RS4: submit is `try_send`, tick is a non-blocking channel receive). Scale control top-right (“Scale ×n” + −/+), clamped to [1, `MAX_GUI_SCALE` = 8] — implementation choice documented in `exportio.rs`; the model enforces ≥ 1. Export prompt widget ids are per-kind (`Id::unique(("path_prompt", kind))`) — fixes a latent S-014 bug where a cancelled prompt leaked its typed text into the next prompt of another kind.
- **Scale is model-level** (FR8): `change_scale` mutates `diagram.scale()`; canvas, `.txt`, SVG and PNG all derive from the same scaled render — no display-zoom path exists.

## Verification

- [x] **AC1** — fake-services flow asserts the sink receives the exporter's exact bytes: `exportio::tests::svg_bytes_are_the_exporters_exact_bytes` + app-level `svg_export_sink_gets_the_exporters_exact_bytes` (file on disk == pinned fake `to_svg` bytes); real-pipeline WYSIWYG pin: `svg_export_matches_the_canvas_grid` (file == `to_svg` of the canvas's own grid). Failure notice: `svg_export_failure_shows_a_notice` (bad sink path → status “SVG export failed: …”, nothing silent).
- [x] **AC2** — fake slow exporter via public `PngJob::from_reply`: `pending_job_stays_pending_until_the_reply_arrives` (Pending → Pending → Done with the exact bytes, terminal after), `failed_job_yields_a_message`, `abandoned_job_yields_a_message`; real threaded worker end-to-end: `real_worker_runs_the_full_flow` + app-level `png_export_completes_through_per_frame_ticks` (“Exporting PNG…” state → per-frame ticks → PNG signature on disk) and `png_export_failure_is_reported_in_the_status`. Code review: the frame path (`tick_png_export`) only does a non-blocking channel receive; rasterization happens exclusively in the worker thread (ADR 012).
- [x] **AC3** — `scale_control_changes_rendered_output_and_clamps`: default ×1; + → rendered canvas 9×5 → 18×10 with different text bytes; SVG bytes differ at ×2 vs ×4; clamped to [1, 8]. Text/SVG/PNG all derive from the same scaled render (structural — one `render()` feeds all three exporters).
- [x] **AC4** — `cargo fmt --all --check` clean; `cargo clippy --workspace --tests` 0 warnings/errors (clippy::all denied); `cargo test --workspace` 344 passed / 0 failed; `cargo tarpaulin --engine Llvm --workspace` **83.97%** ≥ 80% (exportio.rs 100%; uncovered app.rs lines are egui drawing code — the standing headless pattern); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0 (its dead-code warnings are pre-existing until the S-019 entry point).
- [ ] **Visual regression (tier 1/2) — ⚠️ deferred to image rebuild** (same environment gap as S-012…S-017: this container predates the winit X11 client libs `libXcursor.so.1`/`libxi6`/`libxkbcommon0`; apt is blocked by ADR 014's egress allow-list, and the Dockerfile already installs them). After the next image rebuild: (1) `sh .agents/skills/gui-testing/scripts/smoke.sh launch --collections gui-app/tests/fixtures/collections` → expect exit-124 PASS; (2) goldens for the changed toolbar: `smoke.sh capture gui-app/tests/goldens/s015-toolbar.png 3 gui-app/tests/fixtures/one-box.ad --collections gui-app/tests/fixtures/collections` — re-run with a fresh shot, `smoke.sh diff` → expect AE=0. Manual visual check: “Export SVG…” / “Export PNG…” open the prefilled path prompt (sibling of the current file); after OK the status bar reports the saved path; while a PNG job is in flight the button is disabled and the status shows “Exporting PNG…”; “Scale ×n −/+” top-right rescales the canvas live (×1 → ×2 doubles it) and exports follow. 
