---
title: handoff-s006
description: session handoff — S-005 done and committed; what the next session needs to start S-006 (render-pipeline grid)
scope: delivery / render-pipeline
status: stable
updated: 2026-10-25
---

# Handoff: S-005 done → S-006 next

**State:** S-005 committed (`797de88`, "S-005: shape-registry — built-in catalog + template/collection loading"). Board, story file, overview tracker and concept C7/C8 docs all updated in the same commit. Workspace: tests green, clippy/fmt clean under ADR 010, tarpaulin (Llvm) 97.58%.

**Next:** S-006 — `render-pipeline` place → resolve → ResolvedGrid (shapes + text). Board is the source of truth; this note only lists what to read first and the sharp edges.

## Read before implementing (in order)

1. `doc/stories/S-006-render-pipeline-grid.md` — 7 ACs, fixed at breakdown.
2. `doc/arc42/12-concepts/glyph-resolution.md` (C1) — the rule table is still `[TBD]`; **AC3 finalizes it**: per-cell mapping of N/E/S/W connection set + content → exactly one svgbob glyph, priorities cross > T-junction > corner > straight, plus written fallback rules for ambiguous cells. This design work is the heart of the story — do it in the concept doc first, then code to it.
3. `doc/adr/001-glyph-resolution-alphabet.md` (v2) — the svgbob input alphabet is the closed output set (T3); AC7 flips ADR 001 to `accepted` once the per-pattern mapping lands.
4. `doc/arc42/12-concepts/render-pipeline.md` (C2) + §8.4.3 of `doc/arc42/08-building-block-view.md` — pipeline stages, determinism rules (strict row-major, no hash-iteration order in output).
5. `crates/diagram-model/src/objects.rs` — what a `ShapeInstance`/`TextInstance` carries; `crates/shape-registry/src/types.rs` + `builtin.rs` — the catalog API S-006 consumes (`TemplateGrid::get`, `AnchorSet`, built-in 9×5 box/ellipse with n/s/w/e anchors).

## Sharp edges

- **Dependency audit (T5):** `render-pipeline` may depend on `diagram-model` + `shape-registry` only — check the current expected graph in `core-services/tests/dependency_audit.rs` and update it in the same change if S-006 adds edges.
- **Anchors are live cells:** custom template anchor cells must NOT be stamped verbatim — they resolve against connecting edges like built-in borders (S-007 will draw into them; S-006 just leaves them un-stamped / border-like). Non-anchor template cells stamp opaque.
- **Scale (AC5):** `expand(&ResolvedGrid, scale)` is a separate pure function over the already-resolved grid; edges must stay continuous lines at scale > 1 — decide sub-cell placement rules for labels in the C1/C2 docs before coding.
- **Golden files:** AC2/AC3/AC5 want `tests/golden/*.grid` — follow the ad-format golden-file pattern (`crates/ad-format/tests/golden.rs`) for how goldens are asserted.
- Coverage gate: workspace tarpaulin (Llvm) must stay ≥ 80% (currently 97.58%; `gui-app/src/main.rs` is the only other uncovered line).
