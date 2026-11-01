---
title: S-030-diagram-colors-picker
description: CR-009 — diagram colors via color picker (FR15): model + .ad v1.1 + colored SVG/PNG export + canvas WYSIWYG + picker dialog
scope: diagram-model/ad-format/exporters/ui-api/core-services/gui-app
status: done
updated: 2026-11-01
---

# S-030: model + ad-format + exporters + gui-app — diagram colors via color picker (FR15)

## Why

FR15: a diagram has a background color, shape fill color and line/text color — three roles mapping 1:1 onto svgbob `Settings` (`to_svg_with_settings`, verified in the pinned crate source). Chosen with a color picker remembering the last 16 used colors; stored with the diagram (`.ad` 1.1 batch) and shown on canvas + SVG/PNG exports.

## Dependencies

- none beyond done stories (S-008)

## Acceptance criteria

**AC1 — Model (FR15):** `diagram-model` gains `Rgb` (hex parse/format, `#rrggbb`, case-insensitive input, lowercase output), `ColorRole { Background, Fill, LineText }` and `DiagramColors` (one `Option<Rgb>` per role; default = all unset). `Diagram::set_color(role, Option<Rgb>)` + `colors()` accessor; setting one role never touches the others. Text export is untouched — colors are presentation data, not grid content (NFR2).

**AC2 — Format (.ad v1.1):** one optional statement after the header: `colors background #rrggbb fill #rrggbb lines #rrggbb` — roles in any order, each at most once, at least one required; the writer emits it only when any role is set, in canonical order (background, fill, lines), lowercase hex. Duplicate role / unknown role / invalid hex / second `colors` statement → parse error with position. Serializer header bumps to 1.1; every pre-existing 1.0 fixture still parses unchanged and round-trips byte-identically (no golden .ad changes).

**AC3 — Export (FR5/FR15):** the seam's `Exporter::to_svg`/`to_png` take `&DiagramColors`. Unset role → svgbob default for that slot, so an all-default diagram exports **byte-identical** to before (NFR2 pin). Set roles land in the SVG (background / fill / stroke colors) and are rasterized into PNG.

**AC4 — Canvas WYSIWYG:** the canvas background paints the background role's color (else `CANVAS_BG`); every shape body fills with the fill role's color beneath its glyphs; grid glyphs draw in the line & text role's color (else the default). Overlays, grid paper and UI chrome keep their fixed colors.

**AC5 — Picker dialog:** a View ▾ menu item “Colors…” opens a “Diagram colors” window with one row per role: label, current swatch (or “default”), a hex input + Set, and Reset. Setting a color applies live to canvas, preview pane and exports; invalid hex → status-bar error, diagram unchanged; changes report in the status bar (“Background color set to #ff0000” / “… reset”).

**AC6 — Recent colors (FR15):** the last 16 colors used are remembered newest-first with duplicates collapsed, persisted in the app config file (the CR-005 store), and offered as a swatch strip in the dialog; clicking a swatch applies it to the active role (the row last set or focused).

**AC7 — Preview pane:** the SVG preview re-rasterizes when colors change, so the pane matches the FR5 export exactly (S-026 WYSIWYG preserved with colors).

## Implementation notes

- `diagram-model::color`: `Rgb` / `ColorRole` / `DiagramColors`; `Diagram.colors: DiagramColors` field + `set_color`.
- `.ad` v1.1: `colors <role> #hex …` statement (parser lenient on role order; writer canonical). Version bump = MINOR 0 → 1.
- `exporters`: `to_svg_with_colors` / `to_png_with_colors` build svgbob `Settings` from the roles (None → svgbob default slot); legacy `to_svg(grid)` delegates with defaults — byte-stable by construction.
- Seam (`ui-api`): `Exporter::to_svg/to_png(&self, grid, colors)`; `PngWorker::submit(grid, colors)`. ADR 009 → v8 (signature evolution).
- gui-app: View ▾ “Colors…” dialog (hex field + Set/Reset per role, egui swatch button for mouse users); recent strip from `AppConfig.recent_colors` (persisted, ≤16, dedup-front); canvas paints background/fill/glyph colors; preview refresh hash includes the colors.

## Verification

**AC1 — Model:** `diagram-model::color` — `Rgb` (`#rrggbb`; case-insensitive parse, lowercase output), `ColorRole { Background, Fill, LineText }`, `DiagramColors` (one `Option<Rgb>` per role, default all-unset); `Diagram::set_color(role, Option<Rgb>)` + `colors()`; setting one role never touches the others. 4 unit tests.

**AC2 — Format:** `.ad` v1.1 gains `colors <role> "#rrggbb"+` — roles in any order, each at most once, at least one pair required; must precede every object statement, at most one per file. Unknown role, duplicate role in a statement, invalid hex, unquoted hex, empty statement, second statement, and placement after objects all fail with line/column positions (11 parser tests). Writer emits the statement only when any role is set, roles in canonical order (`background`, `fill`, `lines`), lowercase quoted hex (2 serializer tests). Header bumped 1.0 → 1.1: the two golden fixtures were re-saved with the new header only — content unchanged; the byte-identity requirement of `golden_files_are_canonical_and_lossless` forces the fixture to match the writer's own version, and `sample.ad` now pins a canonical `colors` line in the golden round-trip suite. 1.0 backward compatibility is pinned by inline parser tests (`v1_0_file_without_colors_parses` + the pre-existing `ad 1.0` positioned-error suite).

**AC3 — Export:** seam `Exporter::to_svg`/`to_png` take `&DiagramColors` (ADR 009 → v8); svgbob `Settings` slots are mapped only for set roles, so an all-unset diagram exports byte-identical to before (pre-existing exporter goldens unchanged + new byte-stability test). Set roles land in the SVG (background/fill/stroke) and rasterize into PNG; the PNG worker job carries the colors (`PngWorker::submit(grid, colors)`), so queued rasterization matches the SVG export. 3 svg + 1 png tests.

**AC4 — Canvas WYSIWYG:** canvas background paints the background role (else `CANVAS_BG`); shape bodies fill with the fill role beneath their glyphs; grid glyphs draw in the line & text role's color (else default). Overlays, grid paper and UI chrome keep fixed colors. 1 unit test + tier-0b.

**AC5/AC6 — Picker dialog + recents:** View ▾ “Colors…” opens a “Diagram colors” window with one row per role (label, current swatch or “default”, hex field + Set, Reset); setting applies live to canvas, preview and exports and reports in the status bar (“Background color set to #ff0000” / “… reset”); invalid hex → status-bar error, diagram unchanged. Recent strip: last 16 used colors newest-first with duplicates collapsed, persisted in the CR-005 config store (cap/dedup unit tests), swatch click applies it to the active role.

**AC7 — Preview pane:** the S-026 hash gate now includes the diagram colors, so the pane re-rasterizes exactly when they change; tier-0b test pins a refresh after a color change.

**Gates (2026-11-01):** 30 new tests (4 model + 13 format + 4 exporter + 2 config + 1 app unit + 6 tier-0b); Material goldens regenerated for the new View-menu item (diff confined to the menu-bar band). Workspace `cargo fmt` clean, `cargo clippy --workspace --all-targets` 0 warnings, full test suite green; `cargo tarpaulin --engine Llvm --workspace` **93.60%** ≥ 80%; `mkdocs build --strict` clean.
