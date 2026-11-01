---
title: B-006 - ad-forward-reference-lossiness
description: parsing a canonical .ad file fails with UnknownObject when a connector statement precedes the shape it references — parse(serialize(d)) is not lossless for such diagrams (QS7 violation)
scope: ad-format
status: fixed
severity: major
updated: 2026-11-01
---

# B-006: `.ad` forward references break round-tripping

## Summary

The canonical serializer emits objects in stored order, so a diagram whose object list contains a connector **before** the shape it references (legal model state — created e.g. by connecting a standalone free-end connector and later attaching its end to an existing shape, or any attach ordering where the connector was placed first) serializes to a file where the `connect` statement precedes the referenced `shape` statement. The old parser resolved endpoint ids only against statements seen *so far*, so parsing such a canonical file failed with `UnknownObject` — i.e. `parse(serialize(d)) != d`: the format was **not lossless** for legal diagrams, violating QS7 (round-trip stability). Found by S-043's new operation-sequence round-trip property (`ad-format/tests/round_trip.rs`).

## Environment

| | |
|---|---|
| Crate | `ad-format` (`src/parser.rs`) |
| Target | core — platform-independent (affects file save/load on every GUI target) |
| Commit | `d13f957` (S-043 handoff state; bug present since the two were written — serializer in stored order + streaming parser) |
| Other | none |

## Steps to reproduce

Save any diagram where a connector precedes its target shape, or parse this canonical file directly:

```
ad 1.1
connect as c1 from s2.n to 1 1
shape box "" as s2 at 5 5 w 2 h 2
```

## Expected

Per the format contract (QS7 / [diagram-file-format concept](../arc42/12-concepts/diagram-file-format.md)): ids are file-local and a connector end references a *shape object* — with no ordering requirement between statements. The file parses to exactly the diagram that serialized it: `parse(serialize(d)) == d`, and re-serialization is byte-stable.

## Actual

The old streaming parser applied each statement immediately; at line 2, `s2` did not exist yet → parse failed with `UnknownObject(s2)` (reported at the connector statement). Any diagram in this state could not be saved-and-reloaded: the app would refuse to load its own canonical output.

## Evidence

- S-043 property `operation_generated_diagrams_round_trip` (`crates/ad-format/tests/round_trip.rs`) fails on pre-fix code; proptest minimized the failure to an operation sequence whose replay yields a connector-before-shape diagram (seed saved to `round_trip.proptest-regressions` at discovery).
- Deterministic repro: the 3-line file above parses fine by hand reasoning but errors with `UnknownObject(s2)` on pre-fix code.

## Investigation

> Filled while clarifying (S-043 session): dedupe result, docs check, repro attempts. Clarification only — no fix design before handoff.

- **Duplicate:** none — B-001…B-005 are GUI/model defects; none touch the `.ad` parser's reference resolution.
- **Docs check:** the concept doc said a connector end "must reference an existing shape object" without stating *when* it must exist — ambiguous, but the QS7 losslessness requirement (and the serializer's stored-order output) make forward references unavoidable for legal diagrams, so the streaming resolution was wrong, not the docs. Doc updated in the same change to state that ids are file-local and forward references are legal.
- **Repro attempt:** deterministic — parse the 3-line file above on pre-fix code → `UnknownObject(s2)` at line 2. Status: reproduced.

## Fix analysis (2026-11-01, S-043 session)

> Root cause + fix, appended when fixing.

**Root cause.** The parser was single-phase: each object statement was applied to the live `Diagram` immediately, and the model's eager `connect`/`place_*` validation rejects references to ids not yet inserted. The serializer, by contract, emits in stored order — which can place a connector before its target shape. Streaming resolution therefore cannot round-trip every legal diagram.

**Fix (bounded contexts: ad-format + one trusted constructor in diagram-model).** Two-phase parse: `parse_document` now buffers all object statements (`PendingObject` with their line/column), then installs them at once via the new **`Diagram::from_parts(objects, scale, colors)`** — a one-shot trusted constructor that preserves file order (= z-order), validates all aggregate invariants against the *full* list (unique ids, endpoint referential integrity allowing forward references to shapes anywhere in the list, zero-length rule, scale ≥ 1), and rejects non-shape reference targets with `NotAShape`. Deferred model errors are mapped back to their causing statement for positioned parse errors (`error_position`: id-carrying errors point at the declaring statement — the *second* one for duplicates; reference errors at the referencing connector). `scale`/`colors` still stream-validate ("must precede objects" now checks buffered statements, not live model state).

**Pinned by:** parser unit tests `connector_may_reference_a_shape_declared_later` (forward ref legal, file order preserved) and `forward_reference_to_text_is_rejected_at_the_connector` (error positioned at the connector, message "expected a shape") + the S-043 property `operation_generated_diagrams_round_trip` (`parse(serialize(d)) == d` + byte-stable re-save over thousands of random operation sequences).

## Resolution

**Fixed 2026-11-01 in S-043** (commit: see board). The parser is two-phase and forward references are legal; `Diagram::from_parts` is the trusted construction path. Concept doc updated to state ids are file-local and forward references resolve after the full document is read.
