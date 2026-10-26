---
title: handoff-s007
description: session handoff — S-006 done and committed; what the next session needs to start S-007 (render-pipeline connectors)
scope: delivery / render-pipeline
status: stable
updated: 2026-10-26
---

# Handoff: S-006 done → S-007 next

**State:** S-006 committed ("S-006: render-pipeline grid stage — place, resolve, scale"). Board, story file, overview tracker (v0.8), C1/C2 concept docs and ADR 001 (→ accepted v3) all updated in the same commit. Workspace: tests green (22 binaries), clippy/fmt clean under ADR 010, tarpaulin (Llvm) 96.8%.

**Next:** S-007 — anchor-bound connectors in `render-pipeline` + new **ADR 013** for the routing decision. Board is the source of truth; this note only lists what to read first and the sharp edges.

## Read before implementing (in order)

1. `doc/stories/S-007-render-pipeline-connectors.md` — 5 ACs, fixed at breakdown. **AC1 is a design task**: write ADR 013 (routing: straight line between resolved anchor cells vs orthogonal elbows etc.) from `doc/templates/adr.md`, `status: proposed`, link from §8.4.5/C2, add to mkdocs nav.
2. `crates/render-pipeline/src/place.rs` — the stamping stage connectors will join: per-cell `edges` (OR-merged), `content`, `structural_top`. Connector lines are structural stamps; arrowheads (`< > ^ v V`) are content at the target end.
3. `doc/arc42/12-concepts/glyph-resolution.md` (C1, now stable) — rule table + fallbacks F1–F5. **F2 is the connector contract:** edges OR-merge before resolution, so a connector line joining a border cell resolves through the existing 16-row table with no new rules.
4. `crates/diagram-model/src/objects.rs` — `Connector` variant: which fields it carries (source/target object id + anchor name; verify exact shape).
5. `crates/shape-registry/src/types.rs` — `AnchorSet` gives anchor cell offsets per type; built-ins are 9×5 with n/s/w/e edge midpoints. Anchor *cells* in the grid = instance pos + offset.

## Sharp edges

- **Passive anchor base-edges (S-006 gotcha):** pattern-stamped anchors already carry passive base-edge bits from their drawn neighbors (recorded via `stamp_content_with_base_edges` in place.rs). A connector OR-merges into the same cell — check that unbound-anchor content still wins at resolution while bound-anchor edges merge cleanly.
- **Join cells must not stray:** AC2 wants "no stray characters at join cells" — the existing rule table should handle it; if a join produces e.g. `+` where art expects a clean pass-through, that's a C1 gap to record (bump C1 + note in ADR 013 consequences), not a special case in code.
- **Arrowheads are content, not edges:** they sit *inside* the line end (target side) and must stay in the svgbob alphabet (AC4 purity test already covers `< > ^ v V` via `is_svgbob_glyph`).
- **Move-follows-render (AC3):** no stored connector geometry — routing is computed from live anchor cells at render time. The unit test renders `{model + move}` vs moved-model directly and compares bytes.
- **Diagonals:** decide in ADR 013 whether v1 allows diagonal connector lines (svgbob has `/` `\`) or only orthogonal; the story's AC2 mentions diagonal golden files, so if you choose orthogonal-only, record that deviation from AC2 explicitly in the story file's verification (criteria changes must be recorded, not silent).
- **Dependency audit (T5):** no new crate edges expected — verify `core-services/tests/dependency_audit.rs` still passes untouched.
- Coverage gate: workspace tarpaulin (Llvm) ≥ 80% (currently 96.8%).
