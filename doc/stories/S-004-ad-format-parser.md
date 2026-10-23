---
title: S-004 - ad-format-parser
description: .ad parser + canonical serializer — Diagram ↔ text, line/column errors, property round-trip (QS7), golden files
scope: ad-format
status: done
updated: 2026-10-23
---

# S-004: ad-format — parser + canonical serializer

## Why

FR4 (save/load `.ad`) and QS7 (lossless round-trip) are implemented by the Persistence context ([§8.4.4](../arc42/08-building-block-view.md)); this is the Repository of the Diagram aggregate. Implements [ADR 004](../adr/004-diagram-file-format.md) against the S-003 grammar.

## Dependencies

- S-002 — parses into / serializes from the `Diagram` aggregate
- S-003 — frozen grammar to implement

## Acceptance criteria

1. **AC1 — Parse.** `parse(&str) -> Result<Diagram, ParseError>` builds a well-formed aggregate (invariants hold); errors carry line + column. Check: unit tests incl. ≥5 distinct error cases with asserted positions.
2. **AC2 — Serialize.** `serialize(&Diagram) -> String` emits the canonical form: document order preserved, stable spacing/escaping, version header. Check: golden file tests (`tests/golden/*.ad`) for a multi-object diagram incl. text + connectors + scale.
3. **AC3 — Lossless round-trip (QS7).** Property test: for generated well-formed diagrams, `serialize(parse(serialize(d))) == serialize(d)` and the parsed model is equal to `d` (objects, order, geometry, labels, anchors). Check: proptest in `ad-format` passes.
4. **AC4 — Versioning + unknown fields.** Files with a higher minor version parse; unknown statements within major v1 are skipped without error (documented tolerance rule from S-003); unsupported major version → explicit error. Check: unit tests per case.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p ad-format` ≥ 80%.
6. **AC6 — ADR 004 flipped to accepted** (decision implemented; follow-up concept doc exists from S-003); `diagram-file-format.md` Implementation section updated; overview tracker updated.

## Implementation notes

- 

## Verification

- [x] AC1 — `parse_document` builds the aggregate via the `Diagram` API (invariants hold); 20+ unit tests incl. many distinct error cases with asserted line/column (header, version range, int rules, escapes, anchor refs, truncation, unknown statements).
- [x] AC2 — `serialize` emits canonical form; golden tests `tests/golden/sample.ad` (scale + shapes + text + connectors) and `tests/golden/escapes.ad` (quoted strings, backslash/newline escapes) pass byte-for-byte.
- [x] AC3 — proptest `round_trip.rs`: generated well-formed diagrams round-trip byte-stable (`serialize(parse(serialize(d))) == serialize(d)`) with model equality; passes.
- [x] AC4 — higher-minor files parse with unknown statements skipped + preserved verbatim at original position (re-save re-emits the source version so preserved statements stay legal on same-version re-parse); unsupported major → explicit error; unit tests per case.
- [x] AC5 — `cargo fmt --check` clean, `cargo clippy --workspace --all-targets` clean (ADR 010 v4), all workspace tests pass, `cargo tarpaulin --engine Llvm -p ad-format` = 86.36% (≥ 80%).
- [x] AC6 — ADR 004 → accepted (v3); concept doc C6 Implementation section updated; overview tracker + board updated. 
