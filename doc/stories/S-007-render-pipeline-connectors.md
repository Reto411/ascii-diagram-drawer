---
title: S-007 - render-pipeline-connectors
description: anchor-bound connector lines with arrowheads in the resolved grid; straight-line routing decision (new ADR)
scope: render-pipeline
status: done
updated: 2026-10-28
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

- Routing implemented in a dedicated `connectors` module (concept C9) rather than inline in `place.rs`: endpoint resolution (proportional `Rect` mapping, offset `Pattern` mapping), straight/single-corner L decision with deterministic corner choice, structural-edge stamping through the existing `CellState` mechanism, arrowhead as content at the second-to-last route cell.
- Two new typed `RenderError` variants: `UnknownAnchor`, `AnchorOutsideExtent` (the §8.4.1 render-time invariant).
- `is_svgbob_glyph` extended with the ADR 001 v2 arrow set `< > ^ v V` (`V` is membership-only: routing emits lowercase arrows, uppercase appears in user text and must still trigger F4 quote escaping).

## Verification

- [x] AC1 — `doc/adr/013-connector-routing.md` (from template, `status: proposed`, v1) records the routing decision: straight or single-corner L, deterministic corner choice (target-side preference → source-side → fixed default), structural-edge routes joining through the C1 rule table, arrowhead at the target end, proportional Rect anchor mapping, degenerate cases; alternatives (straight-only, multi-corner Manhattan, diagonals, curves) rejected with reasons; linked from §8.4.3 + concept C9 `connector-routing.md` + C2 doc and in mkdocs nav.
- [x] AC2 — `render` draws each connector as svgbob-alphabet edge glyphs between the resolved anchor cells with an arrowhead (`< > ^ v`) at the target end; joins are clean T-junctions via the existing rule table (no stray characters). Golden grids: `connector-horizontal` (straight + T-junctions), `connector-vertical` (ellipse pattern anchors), `connector-diagonal` (single-corner L), `connector-mid-edge` (vertical mid-edge join), `connector-scale-2` (arrowhead sub-cell at scale 2) — all byte-exact, first run.
- [x] AC3 — unit test `connector_follows_moves_and_resizes`: the same final model built directly vs. via `move_object`/`resize` renders byte-identically (connectors store no geometry; anchors re-resolve per render).
- [x] AC4 — S-006 property tests extended: random diagrams now include up to two connectors chaining consecutive boxes with random side anchors; determinism + svgbob-alphabet purity (now incl. arrowheads) still hold; `cargo test -p render-pipeline` green (59 unit + 1 golden + 3 properties).
- [x] AC5 — fmt/clippy (`clippy::all` deny) clean, all 22 workspace test binaries green; `cargo tarpaulin --engine Llvm --workspace`: render-pipeline files 417/431 = 96.75% (connectors.rs 107/108), workspace 1427/1466 = 97.34% — both ≥ 80%. 
