---
title: B-004 - window-edge-resize-not-working
description: on the native (undecorated) window, hovering or dragging the edges/corners shows no resize cursor and does not resize the window — all eight grip zones assumed affected
scope: gui-app
status: fixed
severity: major
updated: 2026-11-01
---

# B-004: Window cannot be resized from its edges or corners on native

## Summary

On the native build, the window cannot be resized by dragging its edges or corners: moving the mouse to the right side or the bottom-right corner does not change the cursor into a resize symbol, and pressing/dragging there does nothing — the window keeps its size. The reporter tried the bottom-right corner and the right edge and assumes all edges/corners are affected. Since [S-027](../stories/S-027-material-design-title-bar.md) made the native window undecorated (ADR 017), these grip zones are the *only* way to resize the window, so this fully blocks window resizing on native.

## Environment

| | |
|---|---|
| OS | Linux, Wayland — KDE Plasma / KWin (undecorated-window resize goes through winit → compositor, so this matters) |
| Target | native |
| Commit | `3480346` |
| Other | none provided |

## Steps to reproduce

1. Launch the native app (`cargo run -p gui-app`, or the RustRover run configuration `.run/gui-app.xml`). The ~1024×768 undecorated window opens with the custom Material app bar at the top (no OS title bar).
2. Move the mouse to the right edge of the window — the outermost ~6 px strip (`RESIZE_GRIP` = 6 pt in `gui-app/src/appbar.rs`) — or to the bottom-right corner square.
3. Observe the cursor, then press and drag outward from that position.

## Expected

Per [ADR 017](../adr/017-material-design-title-bar.md) (undecorated windows "lose native drag-to-move and resize behavior unless re-implemented (app-bar drag region + edge resize)") and [S-027's verification](../stories/S-027-material-design-title-bar.md): the app owns resizing via invisible 6-pt grips along all four edges plus a square at each corner; **hovering a grip shows the matching resize cursor** (e.g. `ResizeEast` on the right edge, `ResizeSouthEast` at the bottom-right corner), and **pressing + dragging resizes the window** in that direction (one-shot `BeginResize(dir)` viewport command → winit's native resize drag).

## Actual

The cursor stays the default arrow — no resize symbol appears on the right edge or the bottom-right corner. Pressing and dragging there does nothing: the window size never changes. Reporter: "When moving the mouse to the corners or the side of the window, it is not resizable, neither does the mouse look like a resizing symbol" (tried bottom-right corner + right side; assumes all zones are affected).

## Evidence

> Public repo: no local paths, usernames, or credentials in any pasted output. Screenshots/logs/repro files live in `assets/B-NNN/`.

- No screenshot or recording provided by the reporter yet (see "Requested from reporter" below).
- Headless state at commit `3480346`: tier-0b test `gui_tests::window_edge_grips_queue_begin_resize_once` **passes** — in the headless egui harness, hovering each of the eight grip zones sets the exact expected resize cursor icon (`FullOutput.platform_output.cursor_icon`) and pressing queues exactly one `BeginResize(dir)`. The defect therefore does not reproduce at the egui level; it only manifests with a real native window (eframe/winit → window manager).

**Provided by reporter:** OS = Linux, Wayland; compositor = KDE (Plasma / KWin). **Still requested (optional):** a short screen recording of hovering the right edge + dragging.

## Investigation

> Filled by the AI while clarifying (bug-report skill): dedupe result, docs check (as-designed detection only), repro attempts with commands + results showing observed vs expected. Clarification only — no root cause analysis, no fix design (those belong to the bug-fix skill at handoff; its findings are appended here as a dated "Fix analysis" block).

- **Duplicate:** none — B-001 is the *shape* resize handle on the canvas doing nothing (render-pipeline `Pattern` stamping); this is the *window's* edge/corner resize.
- **Docs check:** **contradicts** [ADR 017](../adr/017-material-design-title-bar.md) (decision 2 + consequences: undecorated window must re-implement "app-bar drag region + edge resize") and [S-027's verification](../stories/S-027-material-design-title-bar.md) ("`window_edge_grips_queue_begin_resize_once` pins all eight zones — hover shows the correct `ResizeDirection` cursor, press queues exactly one `BeginResize(dir)`"). This is a real bug, not as-designed.
- **Repro attempt:** `cargo test -p gui-app window_edge_grips` at `3480346` → `test gui_tests::window_edge_grips_queue_begin_resize_once ... ok` (1 passed) — the egui-level behavior (cursor icon + one-shot `BeginResize`) is intact in the headless harness, so the defect was **not reproduced headlessly**; it requires a real native window stack. Status stays `open`.

## Fix analysis (2026-11-01, bug-fix session)

> Root cause + fix design, appended by the bug-fix skill at handoff.

**Root cause.** The old grips queued one `ViewportCommand::BeginResize(dir)` per gesture and then *waited for the compositor*: egui-winit forwards it to winit's `drag_resize_window`, which on Wayland (winit 0.30.13) calls xdg-toplevel `resize()` — a compositor-driven pointer grab. That path is only as reliable as the compositor's implementation of CSD resize, and on the reporter's KDE Plasma/KWin it produced nothing (no cursor change, no size change) for this undecorated window. The headless tier-0b tests passed because they assert the *egui-level* contract (cursor icon + command queued), which was intact — the failure lives entirely below egui, in the winit → compositor handoff.

Secondary fragility found during analysis: even where `BeginResize` works, the compositor only starts its grab if the command is processed while the pointer button is still held — a serial/timing coupling that makes the old design inherently flaky across compositors.

**Fix (bounded context: gui-app only).** The app now owns the resize gesture end-to-end (ADR 017 v3):

1. A grip press records `ResizeGrab { direction, start_size, start_outer, start_pointer }` in `DrawingApp` state; no viewport command is queued on press.
2. While the primary button is held (`PointerState::button_down` — note: egui 0.37's `button_pressed` is *edge*-triggered, "pressed this frame"), each frame applies one pure step `resize_step(grab, pointer)`: it computes the new inner size from the pointer delta (clamped to `MIN_WINDOW_SIZE`) and, for west/north directions, the re-anchored outer top-left so the opposite edge/corner stays fixed. Changed values are queued as `ViewportCommand::InnerSize` / `OuterPosition` — commands eframe/winit implement on every supported backend.
3. Release ends the gesture (state dropped); nothing is queued afterwards. The one-shot-per-gesture discipline of S-027 is preserved in form: exactly one active gesture at a time, started by press, ended by release.

**Pinned by:** `resize_step_geometry_per_direction` + `resize_step_clamps_to_the_minimum_window_size` (pure geometry, all 8 directions), `window_edge_grips_show_their_cursor_on_hover` (8 zones), `edge_grip_press_and_drag_resizes_the_window` (press → drag → exact `InnerSize`, release → quiet; asserts `BeginResize` is never queued), `west_and_north_grips_keep_the_opposite_edge_fixed` (new `pump_viewport_at` harness variant reporting a window at a known screen position; asserts the `OuterPosition` re-anchor), and the strengthened `resize_grips_disappear_on_a_degenerate_window`. Workspace: fmt clean, clippy (`clippy::all` deny) 0 errors, all tests green, tarpaulin 92.71%.

**Caveat:** headless tests verify the command stream the app produces, not the compositor's reaction to it. Manual re-verification on the reporter's KWin/Wayland setup is still recommended before considering this fully confirmed.

## Resolution

> Filled when the bug closes: what fixed it (story S-NNN / commit hash), or why it won't be fixed. Empty while open.

**Fixed 2026-11-01.** The grips no longer delegate resizing to the compositor (`BeginResize` → winit `drag_resize_window` → xdg-toplevel `resize`, which did not work on KDE Plasma/KWin). The app owns the gesture: a grip press records direction + starting size/position, and while the button is held each frame queues `InnerSize` with the pointer-driven size (clamped to the minimum window size) plus — for west/north directions — an `OuterPosition` re-anchor that keeps the opposite edge/corner fixed. All eight zones show their resize cursor on hover as before. See the dated Fix analysis above; decision recorded in [ADR 017 v3](../adr/017-material-design-title-bar.md). Manual re-check on the reporter's KWin/Wayland setup recommended.
