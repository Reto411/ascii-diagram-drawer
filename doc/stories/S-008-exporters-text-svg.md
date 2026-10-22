---
title: S-008-exporters-text-svg
description: exporters crate — to_text (grid dump) and to_svg via svgbob::to_svg on the exact grid bytes; pinned SVG goldens
scope: exporters
status: backlog
updated: 2026-10-22
---

# S-008: exporters — text + SVG emission

## Why

FR2 (plain monospace text) and FR5 (SVG export); the resolved grid **is** svgbob input ([ADR 001](../adr/001-glyph-resolution-alphabet.md) v2) so SVG is a direct headless call into the `svgbob` crate — no adapter ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)). Mitigates risk R4 (byte-stable, pinned goldens).

## Dependencies

- S-006 — ResolvedGrid is the single input
- S-007 — SVG goldens should include connectors (arrowheads) for full coverage

## Acceptance criteria

1. **AC1 — Text.** `to_text(&ResolvedGrid) -> String`: UTF-8, LF line endings, exact grid bytes (trailing whitespace trimmed per a written rule in the concept doc). Check: golden tests incl. an empty grid.
2. **AC2 — SVG.** `to_svg(&ResolvedGrid) -> Result<String, ExportError>` calls `svgbob::to_svg` on the exact text bytes; svgbob parse errors surface as `ExportError`, never a panic. Check: unit tests incl. an error case (construct grid via public API or feed svgbob an invalid string through a test seam).
3. **AC3 — Pinned SVG goldens (R4).** Golden files pin the exact SVG output for ≥4 diagrams (box, box+label, junction, connector w/ arrowhead) so svgbob upgrades that change output are caught. Check: `cargo test -p exporters` fails if a golden is edited.
4. **AC4 — Byte-stable.** Re-exporting the same grid twice yields identical bytes; no timestamps/randomness in output. Check: unit test comparing two exports.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p exporters` ≥ 80%.
6. **AC6 — ADR 005 flipped to accepted** (pipeline + svgbob call implemented as decided); C2 doc Implementation section notes the emission home; overview tracker updated.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
