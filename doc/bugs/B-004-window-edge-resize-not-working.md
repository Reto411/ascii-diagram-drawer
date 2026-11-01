---
title: B-004 - window-edge-resize-not-working
description: on the native (undecorated) window, hovering or dragging the edges/corners shows no resize cursor and does not resize the window — all eight grip zones assumed affected
scope: gui-app
status: open
severity: major
updated: 2026-11-01
---

# B-004: Window cannot be resized from its edges or corners on native

## Summary

On the native build, the window cannot be resized by dragging its edges or corners: moving the mouse to the right side or the bottom-right corner does not change the cursor into a resize symbol, and pressing/dragging there does nothing — the window keeps its size. The reporter tried the bottom-right corner and the right edge and assumes all edges/corners are affected. Since [S-027](../stories/S-027-material-design-title-bar.md) made the native window undecorated (ADR 017), these grip zones are the *only* way to resize the window, so this fully blocks window resizing on native.

## Environment

| | |
|---|---|
| OS | unknown (reporter confirmed "native"; OS + display server/WM not given — relevant because undecorated-window resize goes through winit → window manager) |
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

**Requested from reporter:** OS + display server / window manager (e.g. Linux/X11, Linux/Wayland, Windows, macOS), and ideally a short screen recording of hovering the right edge + dragging.

## Investigation

> Filled by the AI while clarifying (bug-report skill): dedupe result, docs check (as-designed detection only), repro attempts with commands + results showing observed vs expected. Clarification only — no root cause analysis, no fix design (those belong to the bug-fix skill at handoff; its findings are appended here as a dated "Fix analysis" block).

- **Duplicate:** none — B-001 is the *shape* resize handle on the canvas doing nothing (render-pipeline `Pattern` stamping); this is the *window's* edge/corner resize.
- **Docs check:** **contradicts** [ADR 017](../adr/017-material-design-title-bar.md) (decision 2 + consequences: undecorated window must re-implement "app-bar drag region + edge resize") and [S-027's verification](../stories/S-027-material-design-title-bar.md) ("`window_edge_grips_queue_begin_resize_once` pins all eight zones — hover shows the correct `ResizeDirection` cursor, press queues exactly one `BeginResize(dir)`"). This is a real bug, not as-designed.
- **Repro attempt:** `cargo test -p gui-app window_edge_grips` at `3480346` → `test gui_tests::window_edge_grips_queue_begin_resize_once ... ok` (1 passed) — the egui-level behavior (cursor icon + one-shot `BeginResize`) is intact in the headless harness, so the defect was **not reproduced headlessly**; it requires a real native window stack. Status stays `open`.

## Resolution

> Filled when the bug closes: what fixed it (story S-NNN / commit hash), or why it won't be fixed. Empty while open.

-
