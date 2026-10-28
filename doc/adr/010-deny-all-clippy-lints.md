---
title: "010 - deny-all-clippy-lints"
description: Clippy lint policy — only the default group (`clippy::all`) is enabled at deny in the whole workspace; violations fail the build, other groups not enforced
scope: tooling
status: accepted
version: 4
updated: 2026-10-23
---

# ADR 010: Clippy Lint Policy — Default Group (`clippy::all`) at Deny

> The file slug `deny-all-clippy-lints` predates v4 and is kept for link stability; the current decision covers only Clippy's default lint group.

## Context

The quality gates (AGENTS.md conventions, [§5.3](../arc42/05-constraints.md), [§6 definition of done](../arc42/06-quality-model.md)) require `cargo clippy` clean with lint violations failing the build. v1–v3 of this ADR enforced the strictest position: *every* Clippy lint group (`all`, `pedantic`, `restriction`, `nursery`, `cargo`) at deny.

In practice that proved too strict for day-to-day development. The first real code (S-002) already required **ten documented workspace-level exceptions**, and per-item `#[allow(...)]` annotations accumulated across test modules (`single_use_fn`, `missing_const_for_fn`, `indexing_slicing`, …). Much of the opt-in inventory polices style preferences rather than defects — it adds friction to every change without advancing the quality goals (Q1 deterministic output, Q5 testable core) — and `nursery` volatility made the toolchain pin a hard requirement.

Decision forced (2026-10-23, human review): **keep lint violations as build errors, but reduce the active set to Clippy's own default group** — a reasonable baseline instead of the full inventory.

## Decision

**Only Clippy's default group (`clippy::all` = `correctness`, `suspicious`, `style`, `complexity`, `perf`) is active in every crate of the workspace, at `deny`. Any violation fails the build.**

Specifics:

- **Scope:** `clippy::all` only. The opt-in groups `pedantic`, `restriction`, `nursery`, and `cargo` are *not* enforced — they remain allow-by-default.
- **Severity:** `deny` — not `warn`. A lint hit is a build error, locally and in CI.
- **Mechanism (as implemented):** Cargo workspace lints — `[workspace.lints.clippy]` in the root `Cargo.toml` sets `all = { level = "deny", priority = -1 }`, and each crate opts in with `[lints] workspace = true`. The explicit negative priority keeps individual `#[allow]` overrides possible (Clippy ≥ 1.93 makes same-priority group/individual lint mixes a hard error). This makes plain `cargo build` / `cargo test` fail on violations, not only a dedicated clippy run.
- **CI (once it exists):** explicit gate `cargo clippy --workspace --all-targets` on top of the normal build/test matrix ([§5](../arc42/05-constraints.md) build-tooling condition).
- **Exceptions:** individual `#[allow(...)]` attributes are permitted with an adjacent one-line justification; they must be kept minimal and reviewed. The ten workspace-level exceptions documented in v2/v3 are **retired** — they existed only to make the all-groups policy buildable — and were removed from `Cargo.toml` with v4.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| All groups at deny (v1–v3 of this ADR) | Proven too strict in practice: ten workspace exceptions plus growing per-item allows for the first real code; restriction/nursery lints police style, not defects; `nursery` volatility made the toolchain pin a hard requirement. Recorded here and reverted by v4. |
| `clippy::all` as `warn` only | Violations would not fail the build and would silently accumulate; the recorded decision is that failing lints must throw errors. |
| `clippy::all` + `pedantic` at deny | Reasonable middle ground, but `pedantic` produced most of the friction (and false positives) with little benefit for this codebase's quality goals; revisit if review starts missing a class of issue that `pedantic` would have caught. |

## Consequences

- **Positive:** the lint policy matches Clippy's own opinionated baseline — active lints catch real defects, not style preferences; much lower per-change friction; no workspace-level exception bookkeeping; the toolchain pin is no longer *required* by this ADR (kept for reproducible builds).
- **Negative / costs:** `pedantic`/`restriction`-class issues (unsafe indexing, unwraps in non-test code, single-use fns, …) are no longer caught automatically — they must be caught in review instead. Existing per-item `#[allow]`s for those lints in the tree are now dead weight (harmless).
- **Follow-ups:**
  - ~~Reduce `[workspace.lints.clippy]` to `all = deny` and drop the ten workspace exceptions~~ — done with v4.
  - Cleanup pass: remove now-unneeded per-item `#[allow(clippy::…)]` attributes on restriction/nursery lints (test modules of `diagram-model`, `ad-format`, `core-services`; `gui-app/src/main.rs`) — optional, no behavior change.
  - Add the `cargo clippy --workspace --all-targets` CI job once a CI provider exists (open item in [§6](../arc42/06-quality-model.md)).
  - If review starts missing a class of issue an opt-in group would have caught, propose re-enabling *that specific lint* individually (with reason) rather than a whole group.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v4 (2026-10-23):** policy reduced from all-groups-deny to `clippy::all`-only deny, per human review — the full lint inventory proved too strict in practice (ten workspace exceptions plus growing per-item allows for little quality gain; lints like `single_use_fn` add friction without catching defects). Violations of `clippy::all` still fail the build. The ten documented workspace-level exceptions are retired and removed from `Cargo.toml`; the toolchain pin stays, but is no longer required by this ADR. File slug kept for link stability.
- **v3 (2026-10-23, S-002):** seven more workspace-level exceptions documented with reasons, all found while writing the first real code (`diagram-model`): `missing_inline_in_public_items`, `question_mark_used`, `missing_trait_methods`, `pub_with_shorthand`, `exhaustive_structs`, `exhaustive_enums`, `pattern_type_mismatch`. The lint policy itself is unchanged (all groups deny); these are the minimal set of allows that real code needs. Verified: clean gate on `diagram-model` with 100% line coverage.
- **v2 (2026-10-22, S-001):** implemented and accepted. Mechanism refined to `{ level = "deny", priority = -1 }` per group (Clippy ≥ 1.93 makes same-priority group/individual lint mixes a hard error), and three workspace-level exceptions documented with reasons: `cargo_common_metadata`, `blanket_clippy_restriction_lints`, `implicit_return`. Verified: clean gate on the scaffold, negative probe confirms a lint violation fails the build.
- **v1 (2026-10-22):** initial decision — all Clippy lint groups at deny in every workspace crate; build fails on any violation; toolchain pinning as the nursery-volatility mitigation.
