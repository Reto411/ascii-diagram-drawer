---
title: "018 - test-quality-tooling"
description: three complementary test-quality layers — proptest invariant properties over random operation sequences, cargo-mutants suite-strength audits, cargo-fuzz parser targets
scope: tooling / quality
status: accepted
version: 2
updated: 2026-11-01
---

# ADR 018: Test quality layers — property-based, mutation, and fuzz testing

## Context

The suite is unit-test + golden-file heavy (Q5, QS family), with two existing proptest suites (ad-format lossless round-trip, render-pipeline purity/determinism). Three classes of weakness remain unaddressed:

1. **Combinatorial model behavior** — hand-written tests cover individual operations, but not *sequences* of move/resize/connect/attach/detach in arbitrary order. The model's cross-object invariants (referential integrity of connector endpoints, uniqueness, zero-length rejection) are only exercised through the specific paths a human thought to test.
2. **Test-suite strength** — nothing verifies that the tests would actually *catch* a weakened validation condition (e.g. a removed bounds check). A green suite can be vacuous for exactly the code that matters most.
3. **Malformed input at the import boundary** — the `.ad` parser and the `.adc` custom-shape import accept arbitrary user files; only fixed invalid-file cases are tested, not adversarial inputs.

Constraints: no GUI/windowing in any new test (Q5); tools must work in the isolated devcontainer (ADR 014 egress allow-list → bake into the image); the existing 80 % coverage gate and `clippy::all`-deny policy stay untouched (ADR 010).

## Decision

Adopt three complementary layers, each targeting a different failure mode:

1. **Property-based testing with `proptest`** (already a workspace dev-dependency of ad-format/render-pipeline): generate random *valid* diagrams and random *sequences of operations* (place/move/resize/delete, connect, attach/detach/free endpoints, bends, tips, scale, colors). The assertions are **invariants**, not particular expected diagrams: model consistency after every successful operation, typed error + unchanged state on every failure, lossless `.ad` round-trip, deterministic alphabet-pure rendering. proptest's shrinking keeps any counterexample minimal.
2. **Mutation testing with `cargo-mutants`**: periodically mutate the validation-heavy code (`diagram-model`, the `.ad` parser) and measure how many mutants the suite kills. Surviving mutants are triaged — killed by added tests or documented as acceptable. This is a *periodic audit*, not a per-commit gate (runtime cost).
3. **Fuzz testing with `cargo-fuzz`** (libFuzzer, requires a pinned nightly toolchain): dedicated targets for the two import surfaces — `parse_ad` and `parse_adc` — feeding arbitrary bytes into the real parsers. Invariant: never panic; plus lossless re-serialization for anything that parses. Fuzz runs are bounded and on-demand; all fuzz run state (corpus, artifacts) stays local to the workspace volume rather than in git — the seeds are incompressible random bytes, and the volume already persists them between sessions.

Tooling is baked into the devcontainer image (`.devcontainer/Dockerfile`): `cargo-mutants`, `cargo-fuzz`, pinned nightly — isolated runs cannot install tools at runtime (ADR 014).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| More hand-written unit tests only | Does not systematically cover operation *combinations*; no feedback on suite strength; fixed invalid-file cases don't scale to adversarial input. |
| `quickcheck` instead of `proptest` | proptest is already in the workspace (ad-format, render-pipeline); one property-testing ecosystem, and its shrinking is stronger for minimal counterexamples. |
| Per-commit mutation gate in CI | cargo-mutants runtime scales with mutants × test-suite; too slow for every commit. Periodic audit + triage gives the same signal at acceptable cost. |
| Raw libFuzzer harness without cargo-fuzz | cargo-fuzz provides corpus management, minimization, and the standard `fuzz/` layout; writing the harness by hand buys nothing here. |

## Consequences

- **Positive:** invariant-level confidence in the model under arbitrary operation sequences; measurable suite strength (kill rate) with actionable survivor triage; parser robustness against malformed input with a corpus that persists on the workspace volume; minimal counterexamples from proptest shrinking.
- **Negative / costs:** image size grows (nightly toolchain + two cargo subcommands); fuzz runs need nightly (`cargo +nightly fuzz …`); mutation runs are slow enough to be on-demand only; `fuzz/` is a separate crate excluded from the main workspace and its T5 dependency audit.
- **Follow-ups:** extend fuzz targets when new import surfaces appear (e.g. ASCII import, deferred FR12); revisit mutant triage after large model changes; consider running the diagram-model mutation baseline as part of S-020's quality measurement.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v2 (2026-11-01):** status → accepted — S-043 delivered all three layers (property suites green, mutation baselines audited with full survivor triage, both fuzz targets clean over 120 s+ runs), tooling baked into the image. Corpus decision changed: run state is no longer committed to git (incompressible seed bytes would bloat the repo; the workspace volume already persists it between sessions) — see `.gitignore`.
- **v1 (2026-11-01):** initial decision — proptest operation-sequence invariants + cargo-mutants periodic audit + cargo-fuzz parser targets; tooling baked into the devcontainer image.
