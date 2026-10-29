---
title: s012-eframe-gotchas
description: eframe/egui 0.37 API notes + devcontainer X11 gap discovered during S-012/S-014 — read before any gui-app work (S-015…)
scope: gui-app / devcontainer
status: stable
updated: 2026-10-29
---

# S-012 gotchas for follow-up GUI stories

## eframe/egui 0.37 API (the crate moved in 0.36+ — ADR-era examples are stale)

- Scene-based app: `impl eframe::App { fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) }`. No more `fn update(&mut self, ctx, frame)`; the root panel is created inside `ui` via `egui::TopBottomPanel::top("…").show(ui.ctx(), |ui| …)`.
- `NativeOptions` / `eframe::run_native` are `#[cfg(not(target_arch = "wasm32"))]` in 0.37 — a wasm build needs the `WebRunner` entry (S-019). The current wasm `main()` is a documented no-op stub; all app modules already compile for wasm (verified with `cargo check --target wasm32-unknown-unknown`).
- **No animation toggle** in egui 0.37 (`Context::set_animations_enabled` is gone). Our UI is static, so settled frames are stable for goldens — no action needed.
- Cell-exact text: measure the glyph advance per frame with `painter.layout("MMMMMMMMMM", FontId::monospace(px), Color32::WHITE)` and divide by 10 — never assume a fixed px/char ratio (font metrics differ). The canvas draws the exact `to_text` bytes at that advance.
- Pointer edges: `ui.interact(area, id, Sense::click)` gives you per-frame press/release/drag; convert them to the pure gesture machine through `DrawingApp::handle_pointer_frame(held, Option<((u32,u32), HitTarget)>)` (S-013 signature — the target comes from one `canvas::hit_target` call per frame) and `handle_key_frame(delete, escape)` for Delete/Escape. All logic stays headless-testable (pattern from S-012/S-013 — keep it).
- **Input routing gotcha:** `ui.interact()` delivers its response only on the frame(s) where the *exact same* `Id` is passed again — and `ui.id()` changes per widget/row, so it cannot be used as the persistent id. Keep a stable `UiId` (e.g. `ctx.data().get_or_insert(Id::new(0))`) in the app and pass it to every `interact` call; otherwise drag frames after the first are silently dropped.
- **Borrow gotcha (S-014):** `ui.ctx()` returns a *reference* — `let ctx = ui.ctx();` at the top of a panel closure keeps an immutable borrow of `ui` alive, and the next `Panel::… .show(ui, …)` / any mutable `ui` use fails with E0502. Call `ui.ctx()` **inline at the point of use** (the temporary dies at the end of the statement) — e.g. `ui.ctx().copy_text(text)` or `.show(ui.ctx(), |ui| …)`. Only bind `let ctx = ui.ctx();` where no mutable `ui` use follows in scope.
- **Status-bar strings:** user-facing notices are built by the pure flow functions (`fileio::open/save/…` return `Result<_, String>` where the `String` *is* the notice) and stored verbatim in `DrawingApp.status` — keeps every message unit-testable without a window.
- **File IO stays behind the seam:** all `.ad` access goes through `Arc<dyn DiagramStore>`; the one sanctioned exception is `fileio::write_text_file` (cfg-native, for the `.txt` sibling output) — never add direct `.ad` file access in views.

## Canvas is model space, not grid space

`ResolvedGrid` carries no origin (the pipeline trims to the occupied extent), and model space is unbounded. `gui-app::canvas::Viewport` maps **model cells** ↔ pixels with a fixed margin; the text bytes are drawn offset by `DrawingApp::content_origin()`. Keep placement/drag math in model coordinates — it stays stable when the trim origin shifts as shapes move.

## Environment: running image lacks winit X11 client libs (fixed in Dockerfile, not yet rebuilt)

- Symptom: `cargo run -p gui-app` under xvfb aborts with `Failed to open X display … x11-dl error: dlopen(…libXcursor.so.1): cannot open shared object file`.
- Missing at runtime: `libXcursor.so.1`, `libXi.so.6`, `libxkbcommon.so.0` (pulled by winit 0.30's `dlopen`). `libxcb1`/`libgl1`/`libegl1`/`libfontconfig1` are present; the image predates them being needed.
- **Fix landed in `.devcontainer/Dockerfile`** (xvfb layer now also installs `libxcursor1 libxi6 libxkbcommon0`). The *running* container cannot apt-install (egress allow-list, ADR 014 — deb.debian.org is not on it), so tier-1 launch-alive + golden capture stay ⚠️ deferred until the image is rebuilt. After rebuild: `sh .agents/skills/gui-testing/scripts/smoke.sh launch` should exit 124 (alive) and `capture gui-app/tests/goldens/empty-canvas.png 3` should produce a real screenshot.
- Don't add deb.debian.org to the squid allow-list for this — it would reopen the whole apt channel that ADR 014 deliberately closed; the Dockerfile is the sanctioned path.

## Test counts / coverage (S-014 baseline)

- gui-app: 53 headless tests (editor 18, canvas 12 incl. hit-target/overlay data, app 10, fileio 13). Logic modules: fileio.rs 57/57 (100%), canvas.rs 79/82 (96%), editor.rs 104/113 (92%). Workspace tarpaulin (Llvm): 88.10%.
- **Tarpaulin gate gotcha:** `cargo tarpaulin --engine Llvm -p gui-app` reports *every compiled crate* but only runs gui-app's tests — dependency crates show up untested and drag the total to ~40–50%, making the 80% floor mathematically unreachable. Gate on `--workspace` (canonical, AGENTS.md) and cite per-module numbers for the new logic.
- Per-crate gui-app coverage looks low because the egui draw glue + `main` bootstrap are window-bound; keep new logic in `editor.rs`/`canvas.rs`-style pure modules so it stays covered.
- Custom shapes are fixed-size in v1 (ADR 007 v3): the resize handle is offered only for unqualified type ids — custom templates are `collection/name`, so `type_id.contains('/')` is the discriminator (the documented id convention, not an accident).
