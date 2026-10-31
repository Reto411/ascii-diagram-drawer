---
title: S-026-svg-preview-pane
description: CR-004 — right-side SVG preview pane (FR6): live view of the exact export bytes, resizable/hideable with a "Preview" bookmark
scope: gui-app + exporters + core-services
status: done
updated: 2026-10-31
---

# S-026: gui-app — right-side SVG preview pane (FR6)

## Why

FR6: users want to see the *effectively rendered* diagram — the exact SVG bytes Export writes — while editing, since the vector interpretation can look different from the character grid. A right-side pane shows `to_svg` output of the current model (same pipeline/bytes as FR5 export), resizable by dragging its border, hideable, re-opened via a 'Preview' bookmark at the window's right edge.

## Dependencies

- none beyond done stories (S-015)

## Acceptance criteria

Written from [CR-004](../crs/CR-004-svg-preview-pane.md)'s "Requested change" + §14 FR6 wording and the CR's recorded assumptions (pane visible by default; refresh throttled to model changes; follows FR8 scale).

- **AC1 — The pane shows exactly what FR5 exports.** With a non-empty diagram, the right-side preview pane displays the current model rasterized from the *exact export bytes*: the same per-frame canvas text that the character canvas draws is funneled through the same `svg_from_text` → usvg/resvg pipeline as `to_svg`/`to_png`, so the preview (incl. FR8 scale) is byte-derived from what Export writes — never an approximation (NFR3). Rasterization re-runs only when the rendered grid actually changes (hash gate on the canvas text); unchanged models cost nothing.
- **AC2 — Resizable by dragging its border.** The pane's left border can be dragged to change its width within 160–900 px (default 380 px); the chosen width persists across frames, and the canvas takes the remaining space. The model itself is never affected by pane geometry.
- **AC3 — Hideable, re-opened via a "Preview" bookmark.** Dragging the border past the minimum collapses the pane (double-clicking the edge also toggles it); the pane is visible by default (CR assumption). While hidden, a visible **"Preview"** bookmark at the right edge of the window re-opens it.
- **AC4 — Graceful states, no regressions.** An empty diagram shows a placeholder (not an error); a rasterization failure or a build without the `raster` feature (`Unsupported`) shows an explanatory message in the pane while the canvas and all other UI keep working. The preview reuses the frame's existing render (no second render path) and uses only the pure-Rust raster dependencies already required by PNG export (wasm-safe, ADR 016). The full test suite passes — chrome-layout expectations are updated only where the new default pane legitimately changes canvas geometry (the CR's "visible by default" assumption).

## Implementation notes

- **Exporters seam (AC1):** new `exporters::text_to_rgba(text) -> RgbaImage` — the same `svg_from_text` → usvg/resvg pipeline as `to_svg`/`to_png`, but taking the exact svgbob input bytes directly so the GUI can preview from the canvas frame it already holds. `RgbaImage { width, height, rgba }` is a plain row-major raster value type; zero-size documents map to a typed `PngEncode` error (never a panic). The pane's pixels are *defined* as the export's pixels: `to_svg(grid) == svg_from_text(to_text(grid))`, and both the preview and the PNG render that one SVG.
- **Seam discipline (ADR 009):** new `core-services/src/preview.rs` re-exports `{ RgbaImage, text_to_rgba }` — `gui-app` never names the raster crates directly; the pane asks the core services, which own the pipeline. No new `ui-api` trait: rasterization is not a swappable backend (one resvg implementation, ADR 016), so a re-export seam keeps the dependency rule without indirection.
- **The pane (AC2/AC3):** egui `SidePanel::right("preview-bookmark")`, resizable by dragging its left border, width clamped 160–900 px (default 380) and persisted across frames in app state. Dragging past the minimum collapses it; while collapsed a compact "Preview" bookmark button at the right edge re-opens it at the last width. Double-clicking the panel edge toggles collapse/expand (`panel_state` + `desired_width` round-trip). The bookmark panel is marked non-resizable — egui 0.37 left/right panels are resizable by default, and its `__resize` handle sits over the content and would swallow clicks meant for the bookmark.
- **Refresh (AC1/AC4):** `refresh_preview(canvas_text)` runs once per frame with the *exact* canvas text the character canvas draws; a `DefaultHasher` hash gate makes unchanged models a no-op (no re-rasterization cost). Empty canvas text → placeholder "Nothing to preview yet" (the empty document is svgbob-legal, so the exporter happily rasterizes its blank default canvas — the GUI gates it instead). Raster failures (`SvgParse`) and `Unsupported` (build without the `raster` feature) surface as an explanatory message inside the pane while every other UI keeps working; a test-only seam `set_preview_problem` injects one deterministically.
- **Texture:** the raster is uploaded once per change via `egui::ColorImage` → `TextureHandle` (replaced on change, old handle dropped) and drawn scaled to fit the pane with `maintain_aspect` — the image is never re-uploaded for unchanged models.

## Verification

- **AC1:** exporters unit tests pin the seam — `text_to_rgba_matches_the_png_export_pixels` (rasterizing the exact canvas text equals decoding the PNG of the same grid, pixel-for-pixel), `text_to_rgba_is_deterministic`, `text_to_rgba_handles_the_empty_document`. Tier-0b `preview_shows_the_raster_once_content_exists`: place a box → the pane shows the image (no placeholder) and the status bar keeps working.
- **AC2:** tier-0b `preview_pane_border_drag_resizes_the_canvas` — dragging the left border 80 px right shrinks the drawn canvas grid by exactly the expected columns, persists across frames; a second drag back restores it. Model untouched (object count unchanged).
- **AC3:** tier-0b `preview_pane_is_visible_by_default_with_a_placeholder` (empty diagram → placeholder text in the pane) and `preview_pane_collapses_via_double_click_and_reopens_via_bookmark` — double-click on the panel edge collapses it (canvas spans full width, bookmark "Preview" appears at the right edge); clicking the bookmark re-opens the pane with its previous width. Collapse is animation-tolerant: the test pumps frames until the `PanelState` settles.
- **AC4:** tier-0b `preview_problem_message_shows_in_the_pane` — an injected raster problem shows the explanation inside the pane while the canvas and toolbar stay interactive; exporters `text_to_rgba_degrades_to_unsupported_without_the_raster_feature` (feature-off build) + the typed-error mapping in `render_pixmap` cover the non-panic contract.
- **Gates:** workspace `cargo fmt --check` clean; `cargo clippy --workspace --all-targets` 0 warnings; full suite green (486 tests, gui-app 188); `cargo tarpaulin --engine Llvm --workspace` **94.23%** ≥ 80%.
- **Gotcha recorded:** egui 0.37 side panels are resizable by default — the panel's own resize handle overlays its content and wins hit-testing; any interactive widget in a bookmark strip needs `.resizable(false)` on that panel (found by the AC3 re-open test failing until fixed). 
