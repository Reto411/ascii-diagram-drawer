---
title: S-043 - quality-testing-property-mutation-fuzz
description: adds the three missing test-quality layers — proptest operation-sequence invariants, cargo-mutants baseline + triage, cargo-fuzz parser targets
scope: diagram-model + ad-format + render-pipeline (tests) | tooling (Dockerfile, AGENTS.md) | docs (ADR 018)
status: in-progress
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

### ⚠️ Handoff state — 2026-11-01 (planning agent deferred by user instruction)

The planning agent started implementing this story and was instructed to **defer**. Everything below is **uncommitted work-in-progress in the working tree** (verified green at handoff except where noted). A working agent resumes from here; do not discard these changes without review — they contain fixes for two real bugs found by the new properties.

**Already implemented (AC1 + AC2a, uncommitted):**

- `crates/diagram-model/src/testgen.rs` (new) — shared proptest module: `Op` enum covering place/move/resize/delete shapes+text, connect, attach/detach/free endpoints, add/move/remove bends, tip styles, scale, colors; `apply()` dispatch, `replay()`, and `invariant_violations()` (the invariant list from AC1). Shared by all three test suites to avoid duplicating the generator.
- `crates/diagram-model/tests/operations.rs` (new) — AC1: 3 properties (`operation_sequences_keep_model_invariants`, `failed_operations_leave_the_diagram_unchanged`, `replaying_the_same_sequence_yields_equal_diagrams`). **Green at handoff.**
- `crates/ad-format/tests/round_trip.rs` (+95) — AC2a operation-sequence round-trip property (`parse(serialize(d)) == d` + byte-stable re-serialization). **Green at handoff.**
- `crates/render-pipeline/tests/properties.rs` (+125) — AC2b operation-sequence render property (no panic; deterministic grid; alphabet purity; typed errors only). **Added but final full-suite verification NOT confirmed at handoff — re-run first.**
- `crates/diagram-model/Cargo.toml` + `Cargo.lock` — proptest dev-dependency.

**Two real bugs found by the new properties (both fixed in the working tree):**

1. **`Diagram::set_endpoint` partial mutation on rejection** (diagram-model, `src/lib.rs`): mutated endpoint state *before* rejecting a zero-length result, leaving invalid state behind on error. Fixed: validate before mutating; regression test added to the lib unit tests. The old test asserted the error but never checked post-rejection state — that's why it survived.
2. **`.ad` forward-reference lossiness** (ad-format, `src/parser.rs`, +148 lines): the serializer emits objects in insertion order, so a connector created before its target shape serializes as a *forward reference*; the old parser resolved references only against earlier statements → `parse(serialize(d))` failed with `UnknownObject`. Fixed with a **two-phase parse** (buffer all statements, then resolve/apply once every id exists) backed by a new one-shot trusted constructor **`Diagram::from_parts`** in diagram-model (full invariant validation, preserves z-order). Forward-reference regression tests added to the parser suite. Green at handoff.

**Open items for the working agent (in order):**

1. Re-run `cargo test --workspace`; confirm render-pipeline properties pass (AC2b). Check `crates/ad-format/tests/round_trip.proptest-regressions` (untracked proptest artifact with the saved failing seed) — confirm the seed now passes, then **delete the file**.
2. **Doc update required for bug #2:** `doc/arc42/12-concepts/diagram-file-format.md` (~line 60) says a connector end "must reference an existing shape object" — update to state that ids are file-local and forward references are legal (resolved after the full document is read). Same commit as the parser change.
3. Consider filing the two bugs in `doc/bugs/` (B-NNN) per the bug-report convention, or record them in Verification as findings of this story — working agent's call; either way they must be documented.
4. AC3: cargo-mutants baselines for `diagram-model/src/lib.rs` + `ad-format/src/parser.rs`, survivor triage table in Verification. (Tool not yet in the devcontainer — see AC5.)
5. AC4: `fuzz/` crate with `parse_ad` + `parse_adc` targets, ≥120 s runs.
6. AC5: Dockerfile layer for cargo-mutants/cargo-fuzz/pinned nightly (ADR 014 bake-in pattern), AGENTS.md commands, ADR 018 → accepted, full standard gates incl. tarpaulin ≥ 80 %.
7. Commit strategy — split the WIP into reviewable commits, e.g.: (a) set_endpoint fix + regression test; (b) from_parts + two-phase parser + forward-ref tests + concept doc update; (c) testgen + the three property suites (+ Cargo.toml/lock). Fill in this file's Verification section as each AC lands.



## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 —
