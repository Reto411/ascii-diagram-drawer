---
title: "016 - png-rasterizer"
description: PNG export rasterizes the exact SVG produced by to_svg via resvg (usvg + tiny-skia), behind a default `raster` cargo feature — pure Rust, deterministic, wasm-compatible
scope: rendering / exporters
status: accepted
version: 1
updated: 2026-10-28
---

# ADR 016: PNG rasterizer (resvg)

> Numbering note: this decision was planned as ADR 014 in the S-009 board entry, but
> 014/015 were taken by later decisions (egress isolation, license). It lands as 016.

## Context

FR5 requires PNG export alongside text/SVG ([arc42 §6 quality model](../arc42/06-quality-model.md), story S-009).
ADR 005 already delegates the grid→vector step to `svgbob` and deliberately left the
rasterizer TBD. Constraints in play:

- **Same source of truth:** PNG must depict exactly what SVG/text export — no second
  layout path that could drift (C2 concept, emit stage).
- **Wasm target (QS4/AC5):** eframe ships the same app to `wasm32-unknown-unknown`
  (ADR 008) — the rasterizer must compile there. No native-only backends, no `unsafe`
  C deps that break wasm.
- **Deterministic bytes (AC4):** re-exporting the same grid must be byte-identical —
  PNGs are compared in tests and shown to users; timestamps or random padding would
  break that.
- **Typed errors, no panics:** the exporters surface `ExportError`; a rasterizer whose
  API is infallible-by-panic is unacceptable.
- **Build-time degradation (AC3):** if no rasterizer backend is available, `to_png`
  must return a typed error — not fail to build or panic.

## Decision

Use **`resvg` 0.49** — the maintained, pure-Rust SVG renderer (usvg frontend +
tiny-skia software backend) — as the PNG rasterizer:

1. `to_png(grid)` first calls `to_svg(grid)`; the PNG is a rasterization of that exact
   document (no separate grid→raster path).
2. Pipeline seam (`png_from_svg`): `usvg::Tree::from_data` → `resvg::render` into a
   `tiny_skia::Pixmap` created at the SVG's declared pixel size → `Pixmap::encode_png`.
3. The whole chain sits behind a **default cargo feature `raster`** in `exporters`
   (pulls `resvg`, `usvg`, `tiny-skia`). Builds with `--no-default-features` keep
   text/SVG working and get `ExportError::Unsupported` from `to_png`.
4. Every fallible step maps to a typed variant: parse → `SvgParse`, encode/zero-size →
   `PngEncode`; the missing-backend case is `Unsupported`. Nothing panics.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Direct grid→raster drawing (skip SVG) | Second layout path — PNG could drift from text/SVG; violates the single-source-of-truth rule of C2/ADR 005 |
| `image` crate + hand-drawn glyphs | No box-drawing glyph rasterization story; we'd re-implement what resvg already does for free |
| Vello / GPU-based renderers | GPU-oriented; weak or no wasm32-unknown-unknown support — fails QS4 (ADR 008) |
| `usvg` alone | It is only the SVG frontend/parser; it has no rasterizer — resvg is usvg *plus* a renderer |
| Hard dependency, no feature | `Unsupported` would become dead code with no build configuration that exercises it; the feature makes AC3 real and keeps wasm/native builds lean when PNG is unwanted |

## Consequences

- **Positive:** deterministic RGBA-8 PNGs (verified byte-identical across calls); decoded
  dimensions always equal the SVG's declared size; pure Rust — verified compiling for
  `wasm32-unknown-unknown`; `Unsupported` has a real, tested build path (`--no-default-features`).
- **Negative / costs:** heavier dependency tree (~40 crates) when `raster` is on — mitigated
  by the feature gate; SVG's font-metric padding means PNG aspect ratio is *not* exactly the
  grid's (documented in S-009 verification).
- **Follow-ups:** S-011 (responsive PNG worker — ADR 012) will call `to_png` off-thread;
  S-015 wires the GUI export dialog. If resvg is ever replaced, only the `png_from_svg`
  seam changes.

## Version history

- **v1 (2026-10-28):** initial decision — resvg 0.49 behind default `raster` feature; implemented in S-009.
