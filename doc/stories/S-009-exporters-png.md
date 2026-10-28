---
title: S-009-exporters-png
description: PNG export via SVG rasterization — rasterizer decision (ADR 016, resvg), wasm-compatible, Err(Unsupported) path
scope: exporters
status: done
updated: 2026-10-28
---

# S-009: exporters — PNG rasterization

## Why

FR5 includes PNG; ADR 005 defers the rasterizer choice to a follow-up decision (candidate: `resvg`). The exporter must stay headless and testable (NFR5) and compile for wasm32 (NFR1); runtime offloading is S-011's job, not this crate's.

## Dependencies

- S-008 — rasterizes the SVG produced by `to_svg`

## Acceptance criteria

1. **AC1 — Rasterizer decision recorded.** New `doc/adr/014-<slug>.md` (from template, `status: proposed`) picks the rasterizer (verify `resvg` compiles for wasm32-unknown-unknown; record why alternatives were rejected); linked from ADR 005 + mkdocs nav.
2. **AC2 — API.** `to_png(&ResolvedGrid) -> Result<Vec<u8>, ExportError>` returns a valid PNG (signature bytes verified by test or decoder round-trip of dimensions). Check: unit test on a small grid; decoded width/height match the grid aspect.
3. **AC3 — Unsupported path.** The `ExportError::Unsupported` variant is constructible and covered by a test (path taken when no rasterizer backend is available at runtime/build). Check: unit test.
4. **AC4 — Deterministic bytes.** Same grid → byte-identical PNG across two calls. Check: unit test.
5. **AC5 — Wasm compatibility.** `cargo check -p exporters --target wasm32-unknown-unknown` exits 0 with the chosen backend.
6. **AC6 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p exporters` ≥ 80%.

## Implementation notes

- 

## Verification (2026-10-28)

- [x] **AC1 — Rasterizer decision recorded.** Landed as **[ADR 016](../adr/016-png-rasterizer.md)** (`status: accepted`, implemented in this story) — planned number 014 was taken by later ADRs (egress isolation, license); the collision is noted in the ADR header. resvg 0.49 chosen over direct grid→raster drawing, `image`-crate hand-drawing, Vello/GPU renderers, usvg-alone, and a hard dependency — all rejected with reasons in the ADR. Linked from ADR 005's follow-ups + mkdocs nav.
- [x] **AC2 — API.** `to_png(&ResolvedGrid) -> Result<Vec<u8>, ExportError>` (`crates/exporters/src/png.rs`). Test `png_signature_and_decoded_dimensions_match_the_svg` verifies the PNG signature bytes *and* decodes with the real `png` crate: bit depth 8, and width/height equal the SVG's declared size (the pixmap is created at exactly that size — faithful rasterization). Note: decoded aspect ≠ grid aspect in general, because svgbob pads with font-metric margins (e.g. a 23×5 grid → 192×96 px); the SVG-declared-size equality is the faithful check.
- [x] **AC3 — Unsupported path.** `ExportError::Unsupported` added; constructible + Display test in every build, and `to_png_degrades_to_unsupported_without_the_raster_feature` proves the real path: the rasterizer sits behind the default `raster` cargo feature (ADR 016), so `cargo test -p exporters --no-default-features` runs 12 tests green with `to_png` returning `Err(Unsupported)` while text/SVG keep working.
- [x] **AC4 — Deterministic bytes.** Test `reexporting_the_same_grid_is_byte_identical`: two `to_png` calls on the same grid are byte-equal (resvg/tiny-skia introduce no timestamps or randomness).
- [x] **AC5 — Wasm compatibility.** `cargo check -p exporters --target wasm32-unknown-unknown` exits 0 with resvg 0.49 on (default features) — verified 2026-10-28.
- [x] **AC6 — Gates green.** fmt/clippy (deny-all)/workspace tests pass; T5 dependency audit passes (new deps are external, dev-deps excluded from the DAG); `cargo tarpaulin --engine Llvm -p exporters` ≥ 80% (see commit).
