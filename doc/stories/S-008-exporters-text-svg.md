---
title: S-008-exporters-text-svg
description: exporters crate — to_text (grid dump) and to_svg via svgbob::to_svg on the exact grid bytes; pinned SVG goldens
scope: exporters
status: done
updated: 2026-10-28
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

- `to_text` (`src/text.rs`) and `to_svg` (`src/svg.rs`) in `crates/exporters`; `ExportError` in `src/lib.rs`.
- SVG input is exactly the `to_text` bytes (unit test asserts both paths agree) — no adapter, per ADR 005.
- svgbob 0.7.6's public API is infallible (no `Result` entry point — verified against crate source), so `ExportError::Svgbob` is a forward-compat seam for future fallible 0.x versions; degrade-not-fail behavior is pinned by test.
- Golden scenarios build real `Diagram`s through the public model/registry API and render them with `render-pipeline` (dev-dependencies only — the T5 dependency audit excludes dev-deps from the runtime DAG).

## Verification

- [x] AC1 — `to_text` joins rows with LF, trims per-line trailing whitespace, no final newline, empty grid → `""`. Written rule recorded in the [render-pipeline concept doc](../arc42/12-concepts/render-pipeline.md) (Emit stage). Golden tests: `tests/golden/*.txt` for box, box+label, junction, connector, **empty**.
- [x] AC2 — `to_svg` funnels through the test seam `svg_from_text(&str)` into `svgbob::to_svg`; unit tests cover the happy path, the exact-bytes agreement with `to_text`, and out-of-domain/degenerate input (empty, spaces-only, garbage) which degrades to literal text instead of failing. No panic path in our code; `ExportError::Svgbob` Display is unit-tested. See Implementation notes for why no live error case exists on svgbob 0.7.6.
- [x] AC3 — 5 SVG goldens (`tests/golden/*.svg`): box, box+label, junction, connector w/ arrowhead, empty. Regeneration only via `UPDATE_GOLDEN=1`; any byte change fails `cargo test -p exporters`.
- [x] AC4 — `reexporting_the_same_grid_is_byte_identical` compares two exports; goldens additionally pin exact bytes across runs.
- [x] AC5 — `cargo fmt --check` + `cargo clippy --workspace --all-targets` clean; `cargo test --workspace` green; `cargo tarpaulin --engine Llvm -p exporters`: all branches covered (lib 3/3, svg 4/4, text 5/5) ≫ 80%.
- [x] AC6 — ADR 005 flipped to `accepted` (with the infallible-API refinement recorded); C2 concept doc updated (text rule + emission home); overview tracker updated. 
