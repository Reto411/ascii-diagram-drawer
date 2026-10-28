---
title: 005 - use-svgbob-as-vector-renderer
description: SVG export delegates grid→vector interpretation to the svgbob crate, consuming the canonical svgbob-compatible ASCII grid directly (no transliteration)
scope: rendering
status: accepted
updated: 2026-10-28
version: 1
---

# ADR 005: Use svgbob as the grid→vector renderer

## Context

The render pipeline's emit stage ([concept C2](../arc42/12-concepts/render-pipeline.md)) needs a character-grid → vector interpreter: per-cell fragments, merging of collinear fragments, promotion to high-level shapes (rects, circles). Building that from scratch is the heaviest part of our rendering work — and [svgbob](https://ivanceras.github.io/svgbob/) already solves it: a published Rust crate (`svgbob` on crates.io, Apache-2.0, v0.7.x) with a trivial API (`to_svg(&str) -> String`), pure-Rust dependencies (nalgebra, parry2d, sauron, pom — no native-only crates visible), and battle-tested use in zola, kroki, typst, and mdbook.

Product direction sharpens this: the tool is a **Visio-like visual editor for SVG-compatible diagrams** — vector rendering should reuse a proven component rather than be reinvented.

One constraint was in play: [ADR 001 v1](001-glyph-resolution-alphabet.md) fixed our text output to **Unicode box-drawing characters** (`─ │ ┌ …`), while svgbob's input alphabet is **ASCII** (`+ - | . , ' \` < > ^ V …`). Originally bridged by a transliteration adapter; ADR 001 v2 later removed the need for it by making svgbob's alphabet canonical.

## Decision

**Depend on the `svgbob` crate as the grid→vector renderer for SVG export.** Specifically:

1. **The canonical resolved grid uses svgbob's input alphabet** ([ADR 001 v2](001-glyph-resolution-alphabet.md)): text output and the SVG input are the *same bytes*.
2. **No transliteration adapter:** `svgbob::to_svg` consumes the resolved grid directly. Labels containing drawing characters are double-quote-escaped in the grid itself (ADR 001 v2), so text and SVG stay identical.
3. **PNG (FR5)** = rasterization of the produced SVG; the rasterizer is a separate follow-up decision (candidate: `resvg` — pure Rust, WASM-compatible).
4. The `svgbob` integration lives in the platform-independent rendering crate (NFR5) and is unit-tested headlessly with golden files (same grid → identical SVG).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Build our own grid→vector interpreter (C2 as originally sketched) | Re-implements solved, battle-tested geometry logic (fragment merging, shape promotion) at real cost and risk; no benefit while svgbob suffices |
| Switch the canonical alphabet to svgbob's ASCII | Rejected at the time as reversing an accepted decision — **later adopted on 2026-10-18 as [ADR 001 v2](001-glyph-resolution-alphabet.md)** after the user requested svgbob-compatible ASCII output |
| Fork/extend svgbob to accept Unicode box-drawing input directly | Upstream divergence and fork maintenance for what a thin, fully testable adapter achieves with less risk (moot after ADR 001 v2 — the alphabets now coincide) |

## Consequences

- **Positive:** vector interpretation comes from a proven, widely used project; much smaller codebase; upstream fixes/features flow in for free; high-quality SVG (clean lines, arcs, promoted shapes) without writing geometry code ourselves.
- **Negative / costs:** coupling to an external 0.x API and roadmap (expect breaking changes); feature ceiling — diagram content outside svgbob's vocabulary degrades to literal text glyphs in the SVG (degradation, not failure).
- **Follow-ups:** ~~verify `svgbob` compiles for `wasm32`~~ — ✅ verified 2026-10-18: `svgbob v0.7.6` and candidate rasterizer `resvg v0.45.1` both compile for `wasm32-unknown-unknown` (rustc 1.98.0, scratch crate, `cargo check` clean); API confirmed as `to_svg(&str) -> String`. Remaining: choose the PNG rasterizer (`resvg` remains the candidate — WASM-compatible per the same check); if a needed feature is missing upstream → evaluate contributing vs. forking; on acceptance, C2's emit stage is final as documented.
- **Accepted 2026-10-28 (S-008):** implemented in `crates/exporters` — `to_svg` passes the exact bytes of `to_text` to `svgbob::to_svg` (no adapter). One refinement recorded from implementation: svgbob 0.7.6's public API is *infallible* (verified against the crate source — no `Result`-returning entry point), so "parse errors" cannot occur today; out-of-domain input degrades to literal text as the Consequences section already states. The `ExportError::Svgbob` variant exists as a forward-compat seam for future fallible upstream versions (0.x coupling) and is pinned by tests.
