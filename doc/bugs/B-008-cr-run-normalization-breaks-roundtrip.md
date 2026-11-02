---
title: B-008 - cr-run-normalization-breaks-roundtrip
description: preserved unknown statements ending in a run of \r (e.g. X\r\r\n) could not round-trip — re-reading strips one more \r, so parse(serialize(d)) != d (QS7 violation on CRLF-pathological input)
scope: ad-format
status: fixed
severity: minor
updated: 2026-11-01
---

# B-008: `\r` runs at line ends break the unknown-statement round trip

## Summary

The format tolerates CRLF input by stripping a trailing `\r` before each line break (Rust's `str::lines()` semantics — one per line). A line terminated by *more* than one `\r` (e.g. `foo\r\r\n`) therefore yields the preserved content `foo\r`. Re-emitting that verbatim plus `\n` produces `foo\r\n`, which a re-read strips to `foo` — so `parse(serialize(d)) != d`: another QS7 losslessness violation, this time at the CRLF-normalization boundary rather than in whitespace handling. Found by S-043's `parse_ad` fuzz target during its first full 120 s run (after B-007 was fixed).

## Environment

| | |
|---|---|
| Crate | `ad-format` (`src/parser.rs`, unknown-statement preservation) |
| Target | core — platform-independent |
| Commit | `9501b72` (post-B-007-fix state at fuzz start) |
| Other | none |

## Steps to reproduce

```
ad 1.3
foo one␍␍        ← "foo one" + two carriage returns before the LF
```

Parse → save → re-parse: the documents differ (`Unknown("foo one\r")` vs `Unknown("foo one")`).

## Expected

Per QS7 and the CRLF rule ("output is LF only"): whatever normalization is applied at read time must be stable under save+re-read — `parse(serialize(d)) == d`.

## Actual

The parser preserved the raw line minus the line break, keeping the surviving `\r`; the serializer re-emitted it verbatim (correctly, since B-007); but the *next* read applied CRLF stripping again and lost that `\r` — a fixed-point mismatch: the normalization is not idempotent on preserved lines.

## Evidence

- Fuzz artifact `fuzz/artifacts/parse_ad/crash-3fa14b5d…` (target `parse_ad`, invariant "round-trip must be lossless (QS7)"), 243 bytes; the minimal repro above is its essence (a line ending in a `\r` run).
- Deterministic: the two-line file above fails round-trip equality on pre-fix code.

## Investigation

> Filled while clarifying (S-043 session): dedupe result, docs check, repro attempts. Clarification only — no fix design before handoff.

- **Duplicate:** none — B-007 is the whitespace-trim defect in the *serializer* (fixed); this is a read-time normalization defect in the *parser* with a different root cause and a different fix location.
- **Docs check:** the spec's CRLF rule ("a trailing `\r` before the line break is stripped") describes exactly one `\r`; longer runs were unspecified, and "output is LF only" rules out re-emitting them doubled. Normalizing runs at read time is the spec-consistent reading — real bug (unspecified behavior that violates QS7).
- **Repro attempt:** deterministic two-line file above fails on pre-fix code. Status: reproduced.

## Fix analysis (2026-11-01, S-043 session)

> Root cause + fix, appended when fixing.

**Root cause.** `str::lines()` strips exactly one trailing `\r` per line; the preserved raw content could therefore still end in `\r`, and verbatim re-emission (required since B-007) makes that `\r` subject to CRLF stripping on the *next* read — the normalization was not idempotent.

**Fix (bounded context: ad-format, parser only).** When preserving an unknown line, strip the whole trailing `\r` run (`raw_line.trim_end_matches('\r')`) — treating any run as line-break decoration. Known statements are unaffected: they are re-serialized canonically anyway, and a stray interior `\r` is a hard parse error (tokenize does not treat `\r` as a separator), so no silent-loss path exists there. Concept doc updated: "any run of trailing `\r` before the line break is stripped."

**Pinned by:** serializer unit test `unknown_statements_normalize_trailing_cr_runs` (byte-exact output + round-trip equality) and the `parse_ad` fuzz target, whose QS7 invariant held across subsequent 120 s runs with zero artifacts.

## Resolution

**Fixed 2026-11-01 in S-043** (commit: see board). Trailing `\r` runs are normalized at read time; the round trip is now a fixed point for all inputs the fuzz target has explored.
