---
title: S-034-prerendered-shape-previews
description: CR-013 — pre-rendered WYSIWYG shape previews (FR16) (skeleton; ACs at implementation)
scope: gui-app
status: done
updated: 2026-11-01
---

# S-034: gui-app — pre-rendered WYSIWYG shape previews (FR16)

## Why

FR16: sidebar shapes are shown as pre-rendered visual previews generated from the same pattern bytes the renderer stamps on placement — what the user sees when picking a shape is exactly what gets placed. Previews stay legible for templates of any size within the sidebar width.

## Dependencies

- none beyond done stories (S-016)

## Acceptance criteria

From [CR-013](../crs/CR-013-prerendered-shape-previews.md) "Requested change" + §14 FR16 ("Shapes in the template sidebar are shown as pre-rendered previews of their glyph pattern, generated from the same pattern bytes the renderer stamps on placement — what the user sees when picking a shape is exactly what gets placed (WYSIWYG). Previews stay legible for templates of any size within the sidebar width."). Conflict check held: GUI-side rendering of `ShapeInfo.preview` only — no seam change, so ADR 009 stays at v8 (no bump needed).

- **AC1 — Pre-rendered preview replaces the snippet.** Every shape row renders its glyph pattern as a pre-rendered visual preview: all rows of `ShapeInfo.preview`, drawn in monospace with the canvas's font size and line packing (one `painter.text` per row, LEFT_TOP, packed at line height — the same rendering path the canvas uses), instead of the old 9 pt name+snippet. *Headless:* tier-0b test — pump the sidebar over a fake catalog; each preview row appears in the frame as its own text shape carrying exactly that row's bytes.
- **AC2 — WYSIWYG: same bytes, same rendering.** For shapes whose natural width fits the available sidebar width, the preview is drawn at the canvas font size with the same monospace family — pixel-identical glyph rendering to what placement stamps (same bytes, same relative metrics). *Headless:* tier-0b test on a small (3×3) shape: its preview rows are drawn with galley text byte-equal to `info.preview` at font size = the canvas `FONT_SIZE`.
- **AC3 — Any template size fits the sidebar width.** When a pattern's natural width (max row length × monospace advance at the base size) exceeds the available width, the whole preview is uniformly scaled down (font size reduced proportionally) so its total width fits; no clipping or overflow. The scaling decision is a pure function of (max columns, available width, base size, base advance). *Headless:* unit tests on the sizing function (fits → base size; overflows → scaled so width fits; degenerate inputs safe) + tier-0b test with a wide fake template asserting the drawn galley width ≤ available width and font size < base.
- **AC4 — Row behavior unchanged.** The name label stays the placement click target; group/subcategory/collapse/filter behavior (CR-011/CR-012) is untouched; collection templates keep their ✎ edit button. *Verification:* the existing sidebar + gui-app suites (S-016/S-017/S-025/S-032/S-033 tests) stay green; Material goldens regenerated with the pixel diff confined to the sidebar band.

## Implementation notes

- 

## Verification

All 4 ACs verified headlessly (no display needed):

- **AC1** — `sidebar::tests::preview_rows_are_prerendered_at_canvas_size` (tier 0b): a shape with the 3-row pattern `["###", "# #", "###"]` produces exactly three preview text shapes in the egui frame, one per row, each carrying that row's exact bytes — including both identical `"###"` rows. The old 9 pt `RichText` snippet is gone (grep: no `.size(9.0)` left in `gui-app/src`).
- **AC2** — same test asserts every preview galley's font size == the canvas `crate::app::FONT_SIZE` (14 pt, now a shared `pub(crate)` constant) for a shape whose natural width fits — same bytes, same monospace family, same per-row `painter.text(LEFT_TOP)` packing at line height as the canvas stamp path in `canvas.rs`.
- **AC3** — pure sizing function `sidebar::preview_font_size` (base size when `max_cols × base_advance ≤ available width`, otherwise uniformly scaled to fit; degenerate inputs return the base size; extreme overflow floors at `MIN_PREVIEW_FONT_SIZE = 2.0`): 3 unit tests (fits/boundary, exact scale-down arithmetic, degenerate/NaN/negative/extreme). Tier-0b `wide_preview_is_scaled_down_to_fit`: a 60-column template (≈480 px natural vs ≈384 px available in the harness) renders both rows with font size < base and galley width ≤ available width.
- **AC4** — full gui-app suite green (258 tests): all S-016/S-017/S-025/S-032/S-033 sidebar + interaction tests pass unchanged (name label still the placement target, ✎ edit button intact, group/collapse/filter behavior untouched). Material goldens regenerated — pixel diff confined to the sidebar band (dark: 1389 px, bbox x:17..101 y:248..432; light: 1385 px, same bbox).

Quality gates: `cargo fmt` clean, `cargo clippy --workspace --all-targets` 0 warnings (deny-all), workspace tests green (614 passed), tarpaulin (Llvm engine) **93.61%** ≥ 80% (+0.02%). No model/format/pipeline/seam change — conflict check held: previews are rendered GUI-side from `ShapeInfo.preview` bytes, so ADR 009 stays at v8 (no bump needed). 
