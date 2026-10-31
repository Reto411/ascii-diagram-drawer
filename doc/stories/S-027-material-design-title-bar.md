---
title: S-027-material-design-title-bar
description: CR-006 — Material Design theme + custom title bar (FR14)
scope: gui-app
status: done
updated: 2026-10-31
---

# S-027: gui-app — Material Design theme + custom title bar (FR14)

## Why

FR14: 'I want to look the application modern.' The GUI is styled according to Material Design (color system, typography, elevation) and native windows run undecorated with a custom Material app bar carrying minimize / fullscreen / close; the editor canvas keeps its monospace WYSIWYG rendering. Decided in [ADR 017](../adr/017-material-design-title-bar.md); wasm keeps the browser's own chrome (NFR1).

## Dependencies

- S-023/S-025/S-026 — style the final layout once (CR-006 decision plan) — all done.

## Acceptance criteria

Written from FR14's §14 wording + [ADR 017](../adr/017-material-design-title-bar.md)'s three decisions; GUI behavior decomposed per the gui-testing skill (tiers 0/0b/2).

- **AC1 — Material theme module.** A new `gui-app/src/theme.rs` builds an egui `Style` from Material Design tokens for both palettes: color roles (primary / on-primary / surface / on-surface / outline), 4dp-based spacing, rounded corners and elevation shadows on surfaces. Unit tests pin the token values and the built style's properties; construction is deterministic (same input → same style).
- **AC2 — Theme application layered over FR13.** The app installs both Material styles via `ctx.set_style_of` so the existing Light / Dark / Native selection drives the palette: Light → light tokens, Dark → dark tokens, Native → egui's system resolution (dark fallback where unknown). Tier-0b: with `Theme::Light` vs `Theme::Dark`, the chrome surfaces (app bar / panels) are painted in the corresponding palette colors; switching theme from Options repaints live.
- **AC3 — Custom app bar with window controls.** A top app-bar surface renders on every target with the diagram title (current file name, else "ascii-diagram-drawer"). On native targets it carries minimize / fullscreen / close buttons: clicking them queues `ViewportCommand::Minimized(true)` / `Fullscreen(bool)` (toggle) / `Close` — asserted from `FullOutput.platform_output.commands`; dragging the bar's empty area queues `StartDrag` (window move). On wasm the controls are hidden (`cfg`) and the bar still renders, so the layout is identical across targets (NFR1).
- **AC4 — Undecorated native window.** The native entry point runs the window without OS decorations (`with_decorations(false)`), so the app bar *is* the window border; the AC3 drag region replaces native move.
- **AC5 — Canvas stays WYSIWYG.** The canvas keeps its own fixed surface and monospace glyph rendering regardless of palette: it paints an explicit dark background fill first (so a light theme can't wash out the fixed `TEXT_COLOR`/`GRID_LINE_COLOR`), and the canvas region of a dump for the same fixture is pixel-identical under both palettes. All existing tier-0b canvas tests keep passing.
- **AC6 — Visual evidence.** Tier-2 software-rasterizer dumps of the restyled app in dark (default states) and light (app bar + chrome); ASCII view + pixel scans verify the app bar geometry/colors, the three window controls, and the Material palette; goldens committed to `gui-app/tests/goldens/` (first golden set).

## Implementation notes

- **Scope (per CR-006 impact):** static Material styling only — no animated ripples (egui has no animation system); icon symbols are plain Unicode glyphs ("─" minimize, "⛶" fullscreen, "✕" close) rather than ASCII-art→SVG assets — the CR allows that asset pipeline ("may"), but text symbols keep zero new dependencies and stay headless-testable.
- **theme.rs:** M3 baseline tokens (light: primary #6750A4 / surface #FEF7FF / on-surface #1D1B20 …; dark: primary #D0BCFF / surface #141218 / on-surface #E6E0E9 …) mapped onto egui `Visuals` (widget fills, selection, hyperlink), `Spacing` on the 4dp grid, corner radii and window/menu shadows for elevation. Typography: eframe's bundled proportional font stands in for Roboto (ADR 017 "best effort"); the canvas monospace is untouched by construction.
- **App bar:** new top panel drawn before the menu-bar panel; full-width `interact` registered *first* (egui gives priority to the last-added widget, so the title/controls added after still win their rects), then title label + native-only control buttons. Fullscreen toggle reads `ViewportInfo::fullscreen` (`None` headless → enters fullscreen).
- **main.rs:** `with_decorations(false)` on the native viewport only (wasm keeps browser chrome — NFR1).
- **Canvas:** explicit `CANVAS_BG` fill as the first draw in `draw_canvas` — required for AC5, since the canvas previously relied on the panel background showing through.

## Verification

All 6 ACs verified. Evidence per AC:

- **AC1 — Material theme module.** `theme.rs` builds both palettes from M3 baseline tokens (light: primary #6750A4 / surface #FEF7FF; dark: primary #D0BCFF / surface #141218, …) mapped onto egui `Visuals` (widget fills, selection, hyperlink), 4dp-grid `Spacing`, corner radii and window/menu elevation shadows. Unit test `theme_applies_material_visuals` pins the token constants and the built style's properties (rounding 4, button radius 32×32, item spacing 4 pt, window roundness 6, both background colors). Deterministic by construction — pure function of no inputs.
- **AC2 — theme application layered over FR13.** `install_material_styles` registers both palettes via `ctx.set_style_of(Light|Dark, …)`, so the existing Light / Dark / Native selection drives the palette (Native → egui's system resolution, dark fallback where unknown). Tier-0b `app_bar_paints_the_active_material_surface`: the default (dark) frame paints the app-bar rect in `DARK.surface`; after `set_theme(Light)` the *next* frame paints `LIGHT.surface` — live repaint from the Options menu.
- **AC3 — custom app bar with window controls.** Tier-0b `window_controls_queue_viewport_commands`: minimize click → `Minimized(true)`, fullscreen click toggles `Fullscreen(bool)` (reads `ViewportInfo::fullscreen`; headless `None` → enter), close click → `Close`. `app_bar_drag_queues_start_drag_exactly_once` is the regression for the user report *“grabbing the window never undrags it”*: the old code queued `StartDrag` on **every** frame while `dragged()`, and egui-winit forwards each command to winit's `drag_window()` — on X11 those pointer grabs stack, so the window kept following the cursor after release. The bar now registers a drag-only sense and queues exactly one `StartDrag` on the press frame (zero during move frames, zero on hover/release). `window_control_glyphs_are_in_the_font` pins the bundled-monospace glyphs (`─` `□` `×`) so the controls can never render as tofu.
- **AC4 — undecorated native window.** `main.rs` sets `with_decorations(false)` on the native viewport only (wasm keeps browser chrome, NFR1); the AC3 drag region plus the edge grips below replace the OS border.
- **AC5 — canvas stays WYSIWYG.** `draw_canvas` paints an explicit fixed `CANVAS_BG` fill first; tier-0b `canvas_is_pixel_identical_under_both_palettes` rasterizes the same `two-boxes-connector.ad` fixture under Dark and Light with the software rasterizer and compares the canvas rect pixel-for-pixel — identical. All pre-existing canvas tier-0b tests pass unchanged.
- **AC6 — visual evidence.** Tier-2 dumps via the software rasterizer: `10-material-dark.png` (default state) plus the light variant; ASCII view + pixel scans confirmed the app-bar fill `[0,0]–[1024,40]`, hairline at y=40, title text, minimize dash / fullscreen square / close ×, and the Material palette in both themes. First golden set committed to `gui-app/tests/goldens/` (`material-dark.png`, `material-light.png`); `material_theme_goldens_match_committed_pngs` decodes the committed PNGs with a pure-std decoder (no new dependency) and asserts pixel-exact equality against a fresh software rasterization, so any chrome drift fails the build; regenerate with `GOLDEN_UPDATE=1`.

**Window resizing (FR14):** an undecorated window has no OS border to grab, so `appbar.rs` registers invisible 6 pt grips along all four screen edges plus one square at each corner in a transparent top-layer `Area`. `window_edge_grips_queue_begin_resize_once` pins all eight zones — hover shows the correct `ResizeDirection` cursor, press queues exactly one `BeginResize(dir)` (same one-shot rule as `StartDrag`); `resize_grips_disappear_on_a_degenerate_window` covers a window narrower than two grip bands (grips unregistered, no commands).

**Gates:** workspace fmt / clippy(0) / test green — 503 tests passed; tarpaulin (Llvm) **94.68%** ≥ 80%; wasm32 check exits 0 (the S-019 `WebRunner` dead-code warnings are pre-existing and documented).
