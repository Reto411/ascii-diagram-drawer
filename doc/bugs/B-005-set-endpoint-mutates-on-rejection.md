---
title: B-005 - set-endpoint-mutates-on-rejection
description: Diagram::set_endpoint mutates the connector's endpoint before validating, so a rejected zero-length operation leaves the diagram in an invalid (zero-length) state
scope: diagram-model
status: fixed
severity: major
updated: 2026-11-01
---

# B-005: `set_endpoint` leaves partial mutation behind when it rejects

## Summary

`Diagram::set_endpoint` assigned the new endpoint to the connector **before** checking the zero-length rule. When the operation was rejected (`ModelError::ZeroLengthConnector`), the error was returned but the connector had already been mutated — the diagram was left holding a zero-length connector, violating the aggregate invariant that both-free ends never share a cell and that rejected operations leave the model unchanged. Found by S-043's new property-based operation-sequence tests (diagram-model `operations.rs`), which assert invariants after *every* operation, successful or not.

## Environment

| | |
|---|---|
| Crate | `diagram-model` (`src/lib.rs`, `Diagram::set_endpoint`) |
| Target | core — platform-independent (affects every GUI target + the `.ad` parser path) |
| Commit | `d13f957` (S-043 handoff state; bug present at least since CR-010 introduced `set_endpoint`) |
| Other | none |

## Steps to reproduce

```rust
// shapes s1, s2 (boxes); connector c1: s1.e -> s2.w
d.set_endpoint(&c1, Target, ConnectorEndpoint::Free(CellPos::new(9, 9)))?;   // detach target
d.set_endpoint(&c1, Target, ConnectorEndpoint::Attached(anchor(s2, "w")))?;  // re-attach
d.set_endpoint(&c1, Source, ConnectorEndpoint::Free(CellPos::new(9, 9)))?;   // detach source to (9,9)

let err = d.set_endpoint(&c1, Target, ConnectorEndpoint::Free(CellPos::new(9, 9)));
// err == Err(ModelError::ZeroLengthConnector(c1)) — correct …
```

## Expected

Per the aggregate contract (documented on `set_endpoint`: "a rejected operation leaves the diagram unchanged"; invariant: both-free ends at distinct cells): after the `Err`, connector `c1` is exactly as before the attempt — target end still attached to `s2.w`.

## Actual

The old code executed `connector.to = endpoint` first and only then called `reject_zero_length`, so on rejection `c1.to` was already `Free(9, 9)` while `c1.from` was also `Free(9, 9)` — a zero-length connector persisted in the diagram despite the error return. Any subsequent render/save of that state would violate QS7-adjacent invariants (the model's own `invariant_violations` checker reports it).

## Evidence

- S-043 property `operation_sequences_keep_model_invariants` (`crates/diagram-model/tests/operations.rs`) fails on pre-fix code with a minimal failing sequence of the shape: connect two shapes, detach both ends to the same cell → invariant "zero-length connector" violation after a rejected operation.
- Pre-fix, the existing unit test `set_endpoint_attaches_detaches_and_enforces_invariants` asserted the `Err(ZeroLengthConnector)` but never inspected post-rejection state — which is why this survived every earlier suite.

## Investigation

> Filled while clarifying (S-043 session): dedupe result, docs check, repro attempts. Clarification only — no fix design before handoff.

- **Duplicate:** none — B-001/B-002 are GUI resize/scale defects; B-003 selection outline; B-004 window edge resize. None touch `diagram-model` aggregate validation.
- **Docs check:** contradicts the documented contract of `set_endpoint` (rejected operation ⇒ diagram unchanged) and the QS family's premise that model invariants hold after any operation sequence. Real bug, not as-designed.
- **Repro attempt:** deterministic Rust repro above; also caught non-deterministically by proptest (seed saved to `operations.proptest-regressions` at discovery). Status: reproduced.

## Fix analysis (2026-11-01, S-043 session)

> Root cause + fix, appended when fixing.

**Root cause.** Mutate-then-validate ordering: `set_endpoint` wrote the prospective endpoint into the connector and only afterwards ran `reject_zero_length`. On rejection the early `return Err(…)` discarded the *result* but not the already-applied mutation — a classic partial-application bug in an aggregate that must be all-or-nothing.

**Fix (bounded context: diagram-model only).** Validate before mutating: compute the prospective `(from, to)` pair for the end being set and run `reject_zero_length` on it *first*; only on success assign the endpoint. All rejection paths (`UnknownObject`, `NotAConnector`, `ZeroLengthConnector`) now leave the aggregate byte-for-byte unchanged.

**Pinned by:** strengthened unit test `set_endpoint_attaches_detaches_and_enforces_invariants` (asserts the target end is still attached to `s2.w` after the rejected zero-length attempt) + the S-043 property suite (`operation_sequences_keep_model_invariants`, `failed_operations_leave_the_diagram_unchanged`) which checks post-operation invariants and clone-before/`PartialEq`-after for every rejected op across thousands of random sequences.

## Resolution

**Fixed 2026-11-01 in S-043** (commit: see board). `set_endpoint` now validates the prospective endpoints before mutating; rejected operations leave the diagram unchanged. Pinned by the strengthened unit test + the new operation-sequence properties.
