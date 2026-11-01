---
title: B-007 - unknown-statement-trim-breaks-roundtrip
description: serializer trimmed preserved unknown statements on save, breaking the documented byte-for-byte contract and making parse(serialize(d)) non-lossless for lines with leading/trailing whitespace (QS7 violation)
scope: ad-format
status: fixed
severity: major
updated: 2026-11-01
---

# B-007: preserved unknown statements are trimmed on save — round-trip broken

## Summary

The format contract says unknown statements from higher minor versions are "skipped and preserved verbatim" and "re-emitted byte-for-byte at their original position between known statements on save". The parser honored that (it stores the raw line, minus the line break), but `serialize_document` emitted `raw_line.trim()` — silently dropping leading/trailing whitespace. Any forward-compatible file whose unknown statements carry such whitespace therefore failed `parse(serialize(d)) == d`: a QS7 losslessness violation on *valid* input. Found by S-043's new `parse_ad` fuzz target (cargo-fuzz/libFuzzer) within the first 10 seconds of its first run.

## Environment

| | |
|---|---|
| Crate | `ad-format` (`src/serializer.rs`, `serialize_document`) |
| Target | core — platform-independent (affects save/load on every GUI target) |
| Commit | `df9f286` (S-043 state at fuzz start; the trim predates S-043 — present since unknown-statement preservation was implemented) |
| Other | none |

## Steps to reproduce

Parse and re-save a forward-compatible file whose unknown statement has trailing whitespace:

```
ad 1.3
g␠            ← "g" + one trailing space
```

## Expected

Per the format contract (verbatim preservation, byte-for-byte re-emission, QS7): the saved file contains the line exactly as read (`g␠`), and `parse(serialize(doc)) == doc`.

## Actual

The saved file contained `g` (whitespace trimmed by `out.push_str(raw_line.trim())`). Re-parsing yields `Unknown("g")` ≠ the original `Unknown("g ")`, so the round trip is not lossless. Same for leading whitespace and interior line layout that survives tokenization (e.g. tabs).

## Evidence

- Fuzz artifact `fuzz/artifacts/parse_ad/crash-9dd158…` (target `parse_ad`, invariant "round-trip must be lossless (QS7)"); minimized to 10 bytes: `"ad"1.3\ng␠`.
- Deterministic repro: the two-line file above — `parse_document` → `serialize_document` → `parse_document` gives unequal documents on pre-fix code (verified by hand before the fix).

## Investigation

> Filled while clarifying (S-043 session): dedupe result, docs check, repro attempts. Clarification only — no fix design before handoff.

- **Duplicate:** none — B-001…B-006 are GUI/model/parser-ordering defects; none touch unknown-statement re-emission.
- **Docs check:** the concept doc is *self-consistent and unambiguous* here: "preserved verbatim" + "re-emitted byte-for-byte". The serializer's `.trim()` contradicts it — a real bug, not an as-designed divergence. (The general "trailing whitespace on statement lines is stripped" rule governs tokenization of known statements; the verbatim rule takes precedence for unknowns.)
- **Repro attempt:** deterministic two-line file above fails the round trip on pre-fix code. Status: reproduced.

## Fix analysis (2026-11-01, S-043 session)

> Root cause + fix, appended when fixing.

**Root cause.** Asymmetric handling of preserved lines: the parser stores the raw line verbatim (`Slot::Unknown(raw_line)`), but the serializer normalized it with `.trim()` before emitting — an undocumented extra step that breaks byte-for-byte re-emission whenever the line has leading/trailing whitespace. (The trim was presumably copied from the canonical-output hygiene applied to *generated* statements, where it is correct.)

**Fix (bounded context: ad-format only).** `serialize_document` now emits the preserved line exactly as stored (`out.push_str(raw_line)`). One-line change; parser untouched (its verbatim preservation already matched the contract). Concept doc clarified that the whitespace-stripping rule applies to known-statement tokenization and that verbatim preservation takes precedence for unknown statements.

**Pinned by:** new serializer unit test `unknown_statements_re_emit_verbatim_including_whitespace` (leading space + trailing space + tab, byte-exact output + round-trip equality) and the `parse_ad` fuzz target itself, whose QS7 invariant now holds across 120 s of libFuzzer runs with zero artifacts.

## Resolution

**Fixed 2026-11-01 in S-043** (commit: see board). Preserved unknown statements re-emit byte-for-byte; the `parse_ad` fuzz target's losslessness invariant is green over its full 120 s run with no artifacts.
