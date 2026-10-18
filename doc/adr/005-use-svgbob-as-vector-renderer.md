---
title: 005 - use-svgbob-as-vector-renderer
description: SVG export delegates grid→vector interpretation to the svgbob crate, bridged by a deterministic Unicode→ASCII transliteration of our resolved grid
scope: rendering
status: proposed
updated: 2026-10-18
supersedes: null
---

# ADR 005: Use svgbob as the grid→vector renderer

## Context

The render pipeline's emit stage ([concept C2](../arc42/12-concepts/render-pipeline.md)) needs a character-grid → vector interpreter: per-cell fragments, merging of collinear fragments, promotion to high-level shapes (rects, circles). Building that from scratch is the heaviest part of our rendering work — and [svgbob](https://ivanceras.github.io/svgbob/) already solves it: a published Rust crate (`svgbob` on crates.io, Apache-2.0, v0.7.x) with a trivial API (`to_svg(&str) -> String`), pure-Rust dependencies (nalgebra, parry2d, sauron, pom — no native-only crates visible), and battle-tested use in zola, kroki, typst, and mdbook.

Product direction sharpens this: the tool is a **Visio-like visual editor for SVG-compatible diagrams** — vector rendering should reuse a proven component rather than be reinvented.

One constraint in play: accepted [ADR 001](001-unicode-glyph-matching-renderer.md) fixes our text output to **Unicode box-drawing characters** (`─ │ ┌ …`), while svgbob's input alphabet is **ASCII** (`+ - | . , ' \` < > ^ V …`). The two alphabets must be bridged without reversing ADR 001.

## Decision

**Depend on the `svgbob` crate as the grid→vector renderer for SVG export.** Specifically:

1. **ADR 001 stays intact.** The canonical resolved grid and the text output remain Unicode box-drawing characters.
2. **Deterministic transliteration adapter (our code, small):** before calling `svgbob::to_svg`, the resolved Unicode grid is mapped 1:1 onto svgbob's ASCII alphabet — `─`→`-`, `│`→`|`, `┌/┐`→`.`, `└`→`` ` ``, `┘`→`'`, junctions (`├ ┤ ┬ ┴ ┼`)→`+`; label text passes through unchanged. The v1 glyph set (ADR 001) has no rounded corners, so nothing is lost; if rounded glyphs are added later, they map onto svgbob's `.` `,` `` ` `` `'` corner characters.
3. **PNG (FR5)** = rasterization of the produced SVG; the rasterizer is a separate follow-up decision (candidate: `resvg` — pure Rust, WASM-compatible).
4. The adapter lives in the platform-independent rendering crate (NFR5) and is unit-tested headlessly with golden files (same grid → identical SVG).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Build our own grid→vector interpreter (C2 as originally sketched) | Re-implements solved, battle-tested geometry logic (fragment merging, shape promotion) at real cost and risk; no benefit while svgbob suffices |
| Switch the canonical alphabet to svgbob's ASCII (supersedes ADR 001) | Reverses an accepted decision: Unicode text output was explicitly chosen ("Unicode output is desired despite the project name"); degrades the core paste-anywhere UX |
| Fork/extend svgbob to accept Unicode box-drawing input directly | Upstream divergence and fork maintenance for what a thin, fully testable adapter achieves with less risk |

## Consequences

- **Positive:** vector interpretation comes from a proven, widely used project; much smaller codebase; upstream fixes/features flow in for free; high-quality SVG (clean lines, arcs, promoted shapes) without writing geometry code ourselves.
- **Negative / costs:** coupling to an external 0.x API and roadmap (expect breaking changes); feature ceiling — diagram content outside svgbob's vocabulary degrades to literal text glyphs in the SVG (degradation, not failure); the transliteration adapter must stay in sync with both the ADR 001 glyph set and the svgbob spec; WASM build compatibility is unverified.
- **Follow-ups:** verify `svgbob` compiles for `wasm32` (NFR1 gate before acceptance); choose the PNG rasterizer; if a needed feature is missing upstream → evaluate contributing vs. forking; on acceptance, C2's emit stage is final as documented.
