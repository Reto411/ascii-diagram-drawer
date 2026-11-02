---
title: S-038-resize-handle-symbol
description: CR-018 — dedicated resize symbol on the resize handle (FR21) (skeleton; ACs at implementation)
scope: gui-app
status: done
updated: 2026-11-02
---

# S-038: gui-app — dedicated resize symbol on the resize handle (FR21)

## Why

FR21: the resize hook is currently a filled square in the anchor-dot color — it does not communicate its purpose, so resize stays undiscoverable. The handle gets a dedicated symbol visually distinct from anchor dots that clearly reads 'drag to resize' (exact glyph/color specified in this story's ACs).

## Dependencies

- none beyond done stories (S-013)

## Acceptance criteria

Written from FR21 (§14, added by CR-018 acceptance) + the CR's requested change.

**AC1 — Dedicated symbol.** The bottom-right resize handle of a single-shape selection is drawn as a **double-headed diagonal arrow** (heads at both ends of the NW–SE cell diagonal — the axis the handle moves along when dragged), centered on the handle cell. It scales with the viewport cell size like the anchor glyphs.

**AC2 — Distinct from anchor dots.** The symbol is drawn in a dedicated `RESIZE_COLOR` (green) and never in `ANCHOR_COLOR`; no filled square marker remains at the handle cell. Anchor arrows, bend handles and selection outlines are unchanged.

**AC3 — Marker/hit parity preserved.** The symbol is centered on exactly the cell `hit_target` answers with a resize gesture for; `canvas.rs` hit-testing and `overlays()` cell decisions are untouched (only pixels in `draw_overlays` change). The symbol appears only for a single-shape selection (S-035 rule) — multi-selections still show anchors without any resize mark.

**AC4 — WYSIWYG + verification.** Overlays never enter the resolved grid: text/SVG/PNG output is byte-identical (no pipeline change). New tier-0b tests pin the symbol's geometry (diagonal line + two arrowheads in `RESIZE_COLOR` centered on the inner bottom-right cell), its absence under multi-selection, and the old amber square's removal; the existing marker/hit-parity test is carried over to the new symbol.

## Implementation notes

- **Symbol** (`gui-app/src/app.rs`, `draw_overlays` → `Overlay::ResizeHandle` arm): a double-headed diagonal arrow along the NW–SE cell diagonal — the axis the handle actually moves along when dragged. Drawn as five line segments (shaft + two chevron heads), all in the new `RESIZE_COLOR = (76, 175, 80)` — deliberately not `ANCHOR_COLOR` (amber), `SELECTION_COLOR` (blue) or either soft green used for ghosts/regions. All dimensions derive from the cell's own diagonal (`d = (cell_w/2, cell_h/2)`: shaft = 70% of it, head length = 42% of the shaft half, head half-width = 55% of the head length), so proportions hold at every zoom level — monospace cells are taller than wide, so sizing off one axis only made the first cut's heads wider than the shaft (caught by pixel inspection of the dump).
- **Why line segments, not a glyph or mesh:** egui 0.37's `Painter` has no triangle primitive (triangles arrive as tessellated meshes), and text-glyph arrows risk font coverage/tofu in headless rasterization. Line segments are crisp at every zoom level and directly inspectable in `FullOutput` shapes — the tests assert on them without pixel decoding.
- **Untouched, by design (AC3):** `canvas.rs` hit-testing, `overlays()` cell decisions, anchor arrows, bend handles, selection outlines. Only pixels change; marker/hit parity holds because the shaft is centered on the same `center_of(col, row)` the old square used.

## Verification

- **AC1** — `resize_symbol_sits_on_the_inner_bottom_right_cell` (tier 0b): places + selects a box via the real UI, extracts all `RESIZE`-colored line segments from `FullOutput`, takes the longest as the shaft and asserts its midpoint sits on the inner bottom-right cell. Pixel inspection of `GUI_DUMP` shot 04 shows the double-headed NW–SE arrow (9×12 px at ×1).
- **AC2** — same test asserts no 2–8 px filled square in `ANCHOR` color remains anywhere (old marker gone); anchor-arrow tests (`anchor_arrows_point_outward_from_the_selection_edges`) still pass unchanged.
- **AC3** — `resize_symbol_absent_for_multi_selection` (tier 0b): two boxes selected via Ctrl+click → zero `RESIZE`-colored segments. `canvas.rs` untouched (git diff confirms only `app.rs` + tests changed in the GUI layer).
- **AC4** — no pipeline/rendering-crate change at all; overlays never enter the resolved grid, so text/SVG/PNG output is byte-identical by construction. All 281 gui-app tests pass (incl. both new ones); workspace `cargo fmt`/`clippy` clean; tarpaulin 92.94% (+0.01%). Material goldens unaffected — no golden scenario shows a single selection. 
