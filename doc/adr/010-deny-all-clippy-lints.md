---
title: "010 - deny-all-clippy-lints"
description: every Clippy lint is enabled at deny level in the whole workspace — any lint violation fails the build
scope: tooling
status: proposed
version: 1
updated: 2026-10-22
---

# ADR 010: All Clippy Lints Active, Failing the Build

## Context

The quality gates recorded so far (AGENTS.md conventions, [§5.3](../arc42/05-constraints.md), [§6 definition of done](../arc42/06-quality-model.md)) require `cargo clippy` with *deny warnings* — which only covers Clippy's warn-by-default groups (`correctness`, `suspicious`, `style`, `complexity`, `perf`). The large majority of Clippy's lint inventory (`pedantic`, `restriction`, `nursery`, `cargo`) is allow-by-default and would never fire.

The project holds itself to a high quality bar (Q1 deterministic output, Q5 testable core) and the Rust workspace does not exist yet — this is the cheapest moment to fix the strictest lint policy: no legacy code will need mass-allowing later. Decision forced: which Clippy lints are active, and at what severity.

## Decision

**All Clippy lints shall be active in every crate of the workspace, and any violation fails the build.**

Specifics:

- **Scope:** all lint groups — the warn-by-default ones (`clippy::all` = correctness/suspicious/style/complexity/perf) *and* the allow-by-default ones (`pedantic`, `restriction`, `nursery`, `cargo`). Effectively every Clippy lint.
- **Severity:** `deny` — not `warn`. A lint hit is a build error, locally and in CI.
- **Mechanism:** Cargo workspace lints (stable since Rust 1.74) — `[workspace.lints.clippy]` in the root `Cargo.toml` sets every group to `"deny"`, and each crate opts in with `[lints] workspace = true`. This makes plain `cargo build` / `cargo test` fail on violations, not only a dedicated clippy run.
- **CI (once it exists):** explicit gate `cargo clippy --workspace --all-targets` on top of the normal build/test matrix ([§5](../arc42/05-constraints.md) build-tooling condition).
- **Exceptions:** individual `#[allow(...)]` attributes are permitted only with an adjacent one-line justification comment; they must be kept minimal and reviewed.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Default groups only (`clippy::all` + `-D warnings`) | Covers only a minority of Clippy's lints; does not meet the stated goal of "all linters active" |
| All groups as `warn`, deny only correctness/suspicious | Most violations would not fail the build and would silently accumulate; weaker than the recorded decision |
| All groups except `nursery` (stable-subset policy) | Reasonable — `nursery` lints change between Rust releases — but the decision explicitly says *all*; volatility is handled by pinning the toolchain instead of dropping a lint group |

## Consequences

- **Positive:** maximum lint coverage from the first line of code; consistent style without extra convention docs; fewer review comments; no later "tighten the policy" mass-refactor.
- **Negative / costs:** `nursery` lints are unstable across Rust releases — a toolchain bump can break the build with zero code changes, so the toolchain must be pinned (`rust-toolchain.toml`) and bumps treated as deliberate changes. `restriction`/`pedantic` will require some justified `#[allow]`s (e.g. in test code: `unwrap_used`, `indexing_slicing`). Higher upfront friction per change.
- **Follow-ups:**
  - Add `[workspace.lints.clippy]` (all groups `"deny"`) + `[lints] workspace = true` per crate when the workspace is scaffolded ([§8](../arc42/08-building-block-view.md)).
  - Pin the toolchain via `rust-toolchain.toml` in the same scaffolding change.
  - Add the `cargo clippy --workspace --all-targets` CI job once a CI provider exists (open item in [§6](../arc42/06-quality-model.md)).
  - Flip `status: proposed` → `accepted` when the workspace lints config lands.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v1 (2026-10-22):** initial decision — all Clippy lint groups at deny in every workspace crate; build fails on any violation; toolchain pinning as the nursery-volatility mitigation.
