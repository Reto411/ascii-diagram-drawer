---
title: B-009 - lone-cr-masks-keyword-silent-preservation
description: a lone trailing \r (not part of CRLF) was absorbed into the last token by the tokenizer, so "colors\r" in a higher-minor file was silently preserved as an unknown statement; the clean re-emitted line then failed to re-parse — parse(serialize(d)) broke QS7
scope: ad-format
status: fixed
severity: minor
updated: 2026-11-01
---

# B-009: lone `\r` at line end masks keywords, silently preserved as unknown

## Summary

Rust's `str::lines()` strips a `\r` only when it is part of a CRLF pair; a *lone* trailing `\r` (e.g. at end-of-file without a final newline) survives into the line. The tokenizer (`tokenize`/`read_word`) did not treat `\r` as whitespace, so it was absorbed into the last token: ` colors\r` tokenized as the single word `colors\r`. That is not equal to the keyword `colors`, so statement dispatch fell through to the unknown branch — and in a higher-minor file (forward-compat mode) the line was **silently preserved** instead of being parsed. The serializer then re-emitted the trimmed, clean line ` colors`, which on re-read *does* tokenize as the keyword `colors` with zero pairs → hard error `'colors' statement is empty`. So a document that parsed successfully failed to re-parse its own canonical serialization: a QS7 losslessness violation. Found by S-043's `parse_ad` fuzz target (third distinct signature, after B-007/B-008).

## Environment

| | |
|---|---|
| Crate | `ad-format` (`src/parser.rs`, tokenizer) |
| Target | core — platform-independent |
| Commit | post-B-007/B-008 fixes (S-043 session) |
| Other | none |

## Steps to reproduce

```
"ad"1.3#d"1.3#\a␊        ← header ad 1.3, rest a comment
"ad"1.8# ␊                ← unknown statement (preserved, legal in 1.3)
 colors␍                  ← "colors" + lone CR, no final LF
```

Parse → OK (the `colors` line silently preserved). Save → re-parse → **error** at the colors line: `'colors' statement is empty — expected at least one <role> "#rrggbb" pair`.

## Expected

Either the line parses as a (truncated) `colors` statement and fails *at parse time*, or it is preserved in a way that re-parses identically. Per QS7: `parse(serialize(d)) == d` for everything that parses.

## Actual

Parse succeeded via silent unknown-preservation; the canonical serialization no longer parsed at all.

## Evidence

- Fuzz artifact `fuzz/artifacts/parse_ad/crash-d04f1ce3…` (target `parse_ad`, invariant "canonical serialization of a parsed document must parse"), 34 bytes — minimal form above.
- Deterministic: the three-line file above reproduces on pre-fix code (repro example run during the S-043 session).

## Investigation

> Filled while clarifying (S-043 session): dedupe result, docs check, repro attempts. Clarification only — no fix design before handoff.

- **Duplicate:** none — B-007 was serializer-side whitespace trimming; B-008 was trailing-`\r`-run preservation. This is a tokenizer defect: `\r` treated as word content.
- **Docs check:** the spec's CRLF rule covers `\r\n` line breaks only; a lone `\r` was unspecified, and silent keyword-masking contradicts the dispatch contract (keywords dispatch on exact token text). Real bug.
- **Repro attempt:** deterministic three-line file above fails on pre-fix code. Status: reproduced.

## Fix analysis (2026-11-01, S-043 session)

> Root cause + fix, appended when fixing.

**Root cause.** `read_word`'s stop set (`' ' | '\t' | '"' | '#' | ','`) and `tokenize`'s skip set did not include `\r`, while `lines()` keeps a lone trailing `\r`. The character leaked into the last token, changing its text and defeating keyword dispatch.

**Fix (bounded context: ad-format, tokenizer only).** Treat `\r` as whitespace in both places: `tokenize` skips it, `read_word` stops at it. Consequences:
- ` colors\r` now tokenizes as `colors` → dispatched to `parse_colors` → hard error *at parse time* (the correct outcome for a truncated colors statement).
- Known statements with stray `\r`s anywhere on the line parse leniently (consistent with CRLF tolerance) instead of producing "not a valid integer"-style errors.
- Interior `\r` in *preserved* unknown lines is content, round-trips verbatim (not adjacent to the line break, so no re-read stripping); trailing runs are still stripped by the B-008 normalization — the two fixes compose.

**Pinned by:** parser unit tests `lone_cr_at_eof_does_not_mask_a_keyword` and `unknown_line_with_interior_cr_round_trips`, plus the `parse_ad` fuzz target (subsequent 120 s runs: zero artifacts).

## Resolution

**Fixed 2026-11-01 in S-043** (commit: see board). `\r` is whitespace to the tokenizer; the crash input now fails at parse time with a positioned error instead of producing an un-re-parsable serialization.
