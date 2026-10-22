---
title: S-006 - render-pipeline-grid
description: place → resolve → ResolvedGrid for shapes + text; C1 glyph-resolution rule table finalized; scale expansion (FR8); golden files
scope: render-pipeline
status: backlog
updated: 2026-10-22
---

# S-006: render-pipeline — place → resolve → ResolvedGrid (shapes + text)

## Why

The [render pipeline](../arc42/12-concepts/render-pipeline.md) (C2) is the single source of truth for all outputs: model → resolved character grid → text/SVG/PNG ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)); [glyph resolution](../arc42/12-concepts/glyph-resolution.md) (C1, ADR 001) makes joins deterministic. Q1/NFR3 determinism and FR7 grid alignment are enforced here. Connectors arrive in S-007; emission lives in `exporters` (S-008).

## Dependencies

- S-002 — consumes the Diagram aggregate
- S-005 — stamps built-in geometry + custom template patterns from the catalog

## Acceptance criteria

1. **AC1 — Pipeline API.** `render(&Diagram, &ShapeRegistry) -> ResolvedGrid` is a pure function (no IO, no RNG); same inputs → byte-identical grid. Check: property test rendering the same model twice + fmt/clippy/test green.
2. **AC2 — Place stage.** Objects stamped at stored coordinates in document order (z-order): built-in box/ellipse borders with centered labels; custom templates as opaque stamps (non-anchor cells verbatim, anchor cells left live); free-text objects. No layout pass anywhere. Check: golden grid files for a mixed diagram (`tests/golden/*.grid`).
3. **AC3 — Resolve stage (C1 finalized).** Per-cell rule table maps the N/E/S/W connection set + content to exactly one svgbob-alphabet glyph, with deterministic priorities (cross > T-junction > corner > straight) and written fallback rules for ambiguous cells (e.g. text under an edge) — recorded in `glyph-resolution.md` (replacing its [TBD]). Check: golden file per pattern incl. every fallback case; unit tests per rule-table row.
4. **AC4 — Alphabet purity (T3).** Property test: every emitted character is in the svgbob input alphabet of ADR 001 v2. Check: proptest in `render-pipeline`.
5. **AC5 — Scale (FR8).** Pure `expand(&ResolvedGrid, scale) -> ResolvedGrid` multiplies cell spacing deterministically (edges stay continuous lines; labels/anchors placed at defined sub-cells); scale 1 = identity. Check: golden files for scale 2 incl. a box + label + junction.
6. **AC6 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p render-pipeline` ≥ 80%.
7. **AC7 — ADR 001 flipped to accepted** (follow-ups done: per-pattern mapping defined, C1 updated); C2 `render-pipeline.md` Implementation section updated; overview tracker updated.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
- [ ] AC7 — 
