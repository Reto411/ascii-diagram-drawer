---
title: S-043 - quality-testing-property-mutation-fuzz
description: adds the three missing test-quality layers — proptest operation-sequence invariants, cargo-mutants baseline + triage, cargo-fuzz parser targets
scope: diagram-model + ad-format + render-pipeline (tests) | tooling (Dockerfile, AGENTS.md) | docs (ADR 018)
status: backlog
updated: 2026-11-01
---

# S-043: Test quality — property-based operation sequences + mutation testing + fuzz testing

## Why

Q5 ("testable core") and the QS family are currently guarded by hand-written unit tests, golden files, and two existing proptest suites (ad-format round-trip, render-pipeline purity/determinism). What is missing: (a) properties over **sequences of operations** — moving, resizing, attaching/detaching connectors — where the assertions are *invariants*, not particular expected diagrams; (b) an audit that the suite actually **detects weakened validation** (mutation testing); (c) defense against **malformed input** to the two import surfaces — the `.ad` parser and `.adc` custom-shape import (fuzzing). Human request 2026-11-01; decision recorded in [ADR 018](../adr/018-test-quality-tooling.md).

## Dependencies

- none — all core crates are complete; independent of S-040/S-041/S-042.

## Acceptance criteria

> Every AC must be verifiable: a command to run, a file/behavior to inspect, or a concrete user action with an observable result. No "works correctly."

1. **AC1 — Operation-sequence invariant properties (diagram-model).** New `crates/diagram-model/tests/operations.rs` (proptest) generates random *sequences* of operations on a live `Diagram` — place/move/resize/delete shapes and text, `connect`, attach/detach/free endpoints via `set_endpoint`, add/move/remove bends, set tip styles, set scale, set colors — and after **every successful operation** asserts the model invariants (unique object ids; attached connector ends reference existing shapes; both-free ends at distinct cells; shape sizes ≥ 1×1; scale ≥ 1; non-empty text) rather than any particular expected diagram. Every failed operation must return a typed `ModelError` and leave the diagram unchanged (clone-before / `PartialEq`-after). Check: `cargo test -p diagram-model --test operations` — at least three distinct properties, all green.
2. **AC2 — Format & render invariants hold on operation-generated diagrams.** (a) `ad-format/tests/round_trip.rs` gains a strategy that builds diagrams through random operation sequences; property: after any sequence, `parse(serialize(d)) == d` and re-serialization is byte-stable (QS7). (b) `render-pipeline/tests/properties.rs` renders operation-sequence diagrams and never panics: on success the grid is deterministic (two renders byte-equal) and alphabet-pure (every cell space or svgbob glyph); on failure only typed `RenderError` variants occur (e.g. `AnchorOutsideExtent` after a resize). Check: `cargo test -p ad-format --test round_trip && cargo test -p render-pipeline --test properties` — green, new tests named to reflect the operation-sequence source.
3. **AC3 — Mutation baseline + survivor triage (cargo-mutants).** `cargo mutants -f crates/diagram-model/src/lib.rs` and `cargo mutants -f crates/ad-format/src/parser.rs` both run to completion; the reports (total / killed / surviving) are recorded in Verification. Every surviving mutant is either **killed by a test added in this story** or explicitly documented as acceptable with a reason (unreachable branch, cosmetic message-only change, …). Check: both commands exit 0 and the triage table exists in this file's Verification section.
4. **AC4 — Fuzz targets for malformed input (cargo-fuzz).** A `fuzz/` crate (own `[workspace]`, not a member of the main workspace) with two targets: `parse_ad` (arbitrary bytes → `ad_format::parse_document`) and `parse_adc` (arbitrary bytes → `shape_registry::parse_collection`). Invariant under any input: **never panics**; for `parse_ad`, additionally, any successful parse re-serializes losslessly (`parse(serialize(doc)) == doc`). Bounded runs of ≥ 120 s each complete with zero crashes, panics, or minimized testcases. Check: `cargo +nightly fuzz run parse_ad -- -max_total_time=120` and the same for `parse_adc` exit 0 with no artifacts under `fuzz/testcases/`.
5. **AC5 — Tooling baked into the devcontainer + docs + standard gates.** `.devcontainer/Dockerfile` gains a layer installing `cargo-mutants`, `cargo-fuzz`, and a pinned nightly toolchain (same bake-in pattern as the wasm-bindgen pin, since isolated runs cannot install at runtime — ADR 014); `AGENTS.md` documents the three commands and when to run them; ADR 018 flips to `accepted`; all standard gates green: `cargo fmt --check`, `cargo clippy --workspace --all-targets` (0 warnings), `cargo test --workspace`, `cargo tarpaulin --engine Llvm --workspace` ≥ 80 %. Check: the gate commands + `grep -n "cargo-mutants\|cargo-fuzz" .devcontainer/Dockerfile AGENTS.md`.

## Implementation notes

> Filled in while working the story: approach, files touched, decisions made (link ADRs), deviations from plan.

- 

## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 —
