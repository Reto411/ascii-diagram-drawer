---
title: S-006 - render-pipeline-grid
description: place → resolve → ResolvedGrid for shapes + text; C1 glyph-resolution rule table finalized; scale expansion (FR8); golden files
scope: render-pipeline
status: done
updated: 2026-10-26
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

- `crates/render-pipeline` now depends only on `diagram-model` + `shape-registry` (T5 audit unchanged). Modules: `edges` (`EdgeSet` bitfield N/E/S/W), `grid` (`ResolvedGrid`, `expand` scale expansion, printable-ASCII invariant), `place` (stamping stage), `resolve` (rule table), `lib` (public API + glyph predicates).
- **Place stage:** each cell tracks merged edge bits, one content char, and a `structural_top` flag implementing the z-order fallbacks: structural stamps claim top on write, content writers clear it (F1 last-writer-wins; F2 edges OR-merge). Built-in `box` = live rectangle border resolved from instance pos/size (`GeometryKind::Rect` — instances of different sizes join cleanly); built-in `ellipse` and all custom templates = opaque pattern stamps (`GeometryKind::Pattern`, ADR 007: non-anchor cells verbatim, space cells transparent per F5) with passive anchor base-edges recorded on drawn anchor cells so S-007 connectors merge with the drawn neighbors. Labels center over the shape interior / full pattern extent (F3 floor-centering + deterministic clipping); lines containing svgbob drawing characters are double-quote-escaped (ADR 001 v2 §4, F4 falls back to the raw line when the quoted form doesn't fit).
- **Resolve stage:** per-cell rule table over the 16 connection sets — straight pairs keep `|`/`-`, every non-straight junction resolves to sharp `+` in v1 (rounded corners/arrows/dashes come from pattern art, not edge resolution; priorities cross > T > corner > straight documented in C1). Content wins over structure unless the cell is structurally top. Grid trimmed to occupied extent, then scale-expanded.
- **Scale (FR8):** `expand` fills line glyphs across their whole scale×scale block (continuity) and places content at one centered sub-cell (top-left bias for even scales); scale 1 = identity.
- Tests: 41 unit tests (incl. all 16 rule-table rows), 11 golden `.grid` scenarios under `tests/golden/` (regenerate with `UPDATE_GOLDEN=1`), 3 proptest properties (determinism, svgbob-alphabet purity, structure-only line-glyph purity).
- Docs: C1 `glyph-resolution.md` finalized (rule table + fallbacks F1–F5, [TBD] removed); C2 `render-pipeline.md` Implementation section filled in; ADR 001 → accepted v3.

## Verification

- [x] AC1 — `render(&Diagram, &ShapeRegistry) -> ResolvedGrid` pure (no IO/RNG/hash-order dependence); proptest `render_is_deterministic` renders random models twice and asserts byte-identical grids; fmt/clippy/test green.
- [x] AC2 — place stage stamps in document order at stored coordinates: box borders + centered labels, ellipse/custom templates as opaque stamps with live anchors, free text; no layout pass. 11 golden files incl. `mixed-diagram.grid` (box + ellipse + quoted text in one render).
- [x] AC3 — rule table maps all 16 connection sets to exactly one svgbob glyph (`rule_table_covers_all_sixteen_states` + per-row tests); priorities cross > T-junction > corner > straight; fallbacks F1–F5 written into `glyph-resolution.md` (TBD removed) with golden files for every case: `cross-junction`, `t-junction`, `content-over-edge`, `edge-over-content`, `label-clipped`, `label-quote-fits`, `label-quote-fallback`.
- [x] AC4 — proptest `emitted_characters_stay_in_the_svgbob_alphabet` over random diagrams (shapes + text incl. drawing-glyph characters, scales 1–3): every emitted char is in the ADR 001 v2 alphabet; plus `structure_only_diagrams_emit_only_line_glyphs`.
- [x] AC5 — `expand(&ResolvedGrid, scale) -> ResolvedGrid`: unit tests for identity (scale 1), continuity, sub-cell placement at scales 2/3; golden `scale-2.grid` with box + label + junction.
- [x] AC6 — fmt/clippy (`clippy::all` deny) clean, all workspace tests green (22 test binaries), render-pipeline files 292/310 = 94.2% line coverage (workspace 96.80%) ≥ 80%.
- [x] AC7 — ADR 001 → accepted (v3); C2 `render-pipeline.md` Implementation section updated; overview tracker + board updated. 
