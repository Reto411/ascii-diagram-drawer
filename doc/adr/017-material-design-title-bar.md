---
title: 017 - material-design-title-bar
description: the GUI is styled according to Material Design; native windows use a custom Material app bar (minimize/fullscreen/close) instead of the OS window border, with the canvas kept monospace for WYSIWYG
scope: gui
status: accepted
updated: 2026-11-01
version: 3
---

# ADR 017: Material Design GUI with custom title bar

## Context

CR-006 (accepted 2026-10-31, FR14): the requester wants the editor to look like a modern desktop application — "a professional tool that looks good" — specifically styled according to **Material Design**, and on native targets the window border replaced by a custom Material app bar with minimize, fullscreen and close controls.

Constraints in play:

- The canvas is WYSIWYG ASCII (ADR 008): its rendering is the resolved grid at measured glyph advance — restyling must not touch it.
- One codebase, native + wasm (NFR1): everything must work on all four targets; a custom title bar only makes sense where the app controls window decorations (native).
- egui/eframe (ADR 008) provides theme primitives (`Visuals::light/dark`, styles, rounding, spacing) but no Material Design system out of the box — colors, typography and elevation must be assembled by hand into an egui `Style`.
- The window-manager interaction (decoration-less window + custom buttons calling minimize/fullscreen/close) is eframe/winit API territory; on wasm there is no window border to replace.

## Decision

1. **Material Design as a theme layer.** A dedicated module in `gui-app` builds the egui `Style`/`Visuals` from Material's design tokens: color system (primary/surface/on-surface roles for light and dark), Roboto-family typography where the platform font allows (falling back to the default UI font otherwise — the canvas keeps its monospace font regardless), 4dp-based spacing, and elevation expressed as egui shadows/rounding on surfaces (cards, menus, dialogs). The theme layer sits above the existing light/dark/native theme selection (FR13): "native" maps to Material's system-aware light/dark, so no third visual mode is introduced.
2. **Custom title bar on native targets.** The window runs undecorated; a top app-bar surface drawn by the app carries the diagram title and the window controls — minimize, fullscreen toggle, close — laid out left-to-right in that order (close at the top-right corner, per KDE/Windows convention; CR-019), wired through eframe/winit window commands. On the wasm target the app bar still renders (title + no-op or hidden window controls) so the layout is identical across targets (NFR1).
3. **Canvas untouched.** The editor canvas keeps its monospace rendering and graph-paper chrome; Material styling applies to chrome only (app bar, menu bar, sidebars, status bar, dialogs, preview pane).
4. **Client-side window resize (v3).** The edge/corner grips do *not* delegate resizing to the compositor. A grip press records direction + starting size/position in app state; while the button is held, each frame computes the new inner size from the pointer delta (clamped to the minimum window size) and queues `ViewportCommand::InnerSize`, plus — for west/north directions — an `OuterPosition` re-anchor so the opposite edge/corner stays fixed. Motivation: the original one-shot `BeginResize(dir)` path (winit → xdg-toplevel `resize()`) is compositor-driven and did not work on KDE Plasma/KWin (B-004), while `InnerSize`/`OuterPosition` are implemented by eframe/winit on every supported backend and are fully testable headlessly.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Keep the OS window border everywhere | Directly contradicts the request; a custom app bar is the visible centerpiece of CR-006 |
| Adopt a Material-Design egui crate (e.g. community `egui_material`-style themes) | External theme crates churn with egui releases and would style internals we don't control; assembling tokens into one module keeps the dependency surface at zero and the look reviewable |
| Full re-skin including the canvas (non-monospace rendering of shapes) | Destroys WYSIWYG (ADR 008) and the text/SVG identity guarantee — the product's core promise |

## Consequences

- **Positive:** consistent modern look across all chrome; native windows feel like a first-party app; one theming module is the single place to adjust tokens.
- **Negative / costs:** undecorated windows lose native drag-to-move and resize behavior unless re-implemented — drag via the app-bar region (`StartDrag`), resize via the client-side grip gesture (decision 4); fullscreen/minimize/close must be wired per platform through winit; font substitution may differ across OSes, so pixel-exact Material typography is a best effort; wasm keeps the browser's own chrome (no-op controls). Client-side resize means size updates arrive per frame while dragging (smooth on modern compositors; on a laggy one the window may feel stepped — acceptable for an editor tool).
- **Follow-ups:** S-027 implements this ADR; if the undecorated-window UX proves rough on any platform, fall back to decorated windows + in-app app bar there only (per-target cfg), recorded in the story's Implementation notes.

## Version history

- **v1 (2026-10-31):** initial decision — accepted with CR-006; status `proposed` until S-027 implements it.
- **Accepted 2026-10-31:** implemented by S-027 (`theme.rs` + `appbar.rs`, undecorated native window, one-`StartDrag`-per-gesture fix, edge resize grips). Decision unchanged — no version bump.
- **v2 (2026-11-01):** CR-019 accepted — the visual *order* of the app-bar controls is now part of the decision: minimize, fullscreen, close left-to-right, so close sits at the top-right corner. FR14 updated to match; S-040 implements it.
- **v3 (2026-11-01):** B-004 — the compositor-driven `BeginResize` path did not work on KDE Plasma/KWin (Wayland): the egui-level contract (hover cursor + queued command) was intact, but no resize ever happened in a real window. Decision 4 replaces it with a client-side resize gesture (`InnerSize` per held frame + `OuterPosition` re-anchor for west/north). The one-shot-per-gesture discipline survives as "one active gesture at a time, started by press, ended by release".
