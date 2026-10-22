---
title: S-007 - render-pipeline-connectors
description: anchor-bound connector lines with arrowheads in the resolved grid; straight-line routing decision (new ADR)
scope: render-pipeline
status: backlog
updated: 2026-10-22
---

# S-007: render-pipeline — anchor-bound connectors

## Why

FR9: connectors bound to object anchors, re-resolved when a shape moves/resizes, so text and vector output stay in sync (C2 relations). The connector *routing* strategy is an open design decision that needs its own record.

## Dependencies

- S-006 — place/resolve stages + ResolvedGrid to draw into
- S-005 — anchor positions come from the catalog's per-type anchor sets

## Acceptance criteria

1. **AC1 — ADR for routing.** New `doc/adr/013-<slug>.md` (from template, `status: proposed`) records the connector routing decision (straight line between resolved anchor cells vs alternatives, e.g. orthogonal elbows), with consequences; linked from §8.4.5 / C2 doc and added to mkdocs nav.
2. **AC2 — Rendering.** `render` draws each connector as a line of svgbob-alphabet edge glyphs between its two anchors' resolved cells, with an arrowhead glyph (`< > ^ v V`) at the target end; joins cleanly into shape borders via the existing resolution rules (no stray characters at join cells). Check: golden grid files for horizontal/vertical/diagonal connectors + a connector joining mid-edge.
3. **AC3 — Follows moves.** Rendering `{model, then move_object}` is byte-identical to rendering the moved model directly (anchors re-resolve, no stored geometry to go stale). Check: unit test comparing both renders.
4. **AC4 — Determinism + purity retained.** AC1/AC4 property tests of S-006 still pass with connectors present; alphabet purity holds for arrowheads. Check: `cargo test -p render-pipeline`.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p render-pipeline` ≥ 80%.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
