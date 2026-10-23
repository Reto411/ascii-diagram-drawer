---
title: "010 - deny-all-clippy-lints"
description: every Clippy lint is enabled at deny level in the whole workspace — any lint violation fails the build
scope: tooling
status: accepted
version: 3
updated: 2026-10-23
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
- **Mechanism (as implemented, S-001):** Cargo workspace lints — `[workspace.lints.clippy]` in the root `Cargo.toml` sets every group to `{ level = "deny", priority = -1 }`, and each crate opts in with `[lints] workspace = true`. The explicit negative priority is required by Clippy ≥ 1.93: a group deny at the default priority (0) would make any individual `allow` a *priority conflict* error, so it could never be overridden. This makes plain `cargo build` / `cargo test` fail on violations, not only a dedicated clippy run.
- **CI (once it exists):** explicit gate `cargo clippy --workspace --all-targets` on top of the normal build/test matrix ([§5](../arc42/05-constraints.md) build-tooling condition).
- **Exceptions:** individual `#[allow(...)]` attributes are permitted only with an adjacent one-line justification (Clippy itself enforces this via `clippy::restriction`: an `allow` without a `reason = "..."` is a build error); they must be kept minimal and reviewed.
- **Documented workspace-level exceptions** (all recorded in the root `Cargo.toml`, each with its reason — no other allows are permitted at workspace level):
  | Exception | Reason |
  |-----------|--------|
  | `cargo_common_metadata = "allow"` | The workspace is an internal, unpublished tool; crates.io packaging metadata (repository/keywords/categories/readme) is not applicable. |
  | `blanket_clippy_restriction_lints = "allow"` | Clippy's own meta-lint discourages enabling the *entire* `restriction` group; this project decision explicitly does so, per ADR 010's goal of "all lints active". The meta-lint is suppressed only to keep that decision buildable — individual restriction lints remain deny. |
  | `implicit_return = "allow"` | Direct conflict with `style::needless_return` (both fire on the same code; Clippy cannot have both active). We keep `needless_return` (catches genuinely redundant returns) and accept implicit trailing expressions, which are idiomatic Rust. |
  | `missing_inline_in_public_items = "allow"` | `#[inline]` is a profile-driven optimization decision, not a style rule; forcing it on every public API function bloats code size and misleads the linker/LTO. |
  | `question_mark_used = "allow"` | `?` is the idiomatic error-propagation mechanism in Rust; forbidding it turns every fallible function into match/if-let boilerplate without improving safety. |
  | `missing_trait_methods = "allow"` | `std::error::Error`'s default `source`/`cause`/`description` implementations are correct as-is; explicit implementations are noise and `description()` is deprecated. |
  | `pub_with_shorthand = "allow"` | Conflicts with `pub_without_shorthand` (both fire, on opposite syntaxes); the project standardizes on the ecosystem-conventional `pub(crate)` shorthand. |
  | `exhaustive_structs = "allow"` / `exhaustive_enums = "allow"` | These lints protect the public API of *published* crates; nothing in this workspace is published to crates.io, and exhaustive construction/matching keeps the model ergonomic for sibling crates (e.g. `match object { Object::Shape(s) => .. }`). |
  | `pattern_type_mismatch = "allow"` | Conflicts with `needless_borrowed_reference` + `ref_patterns`: for non-`Copy` data behind a reference, no match form satisfies all three (explicit `&` patterns need `ref` bindings to avoid moves). Match ergonomics is the idiomatic form; allowed here. |

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
  - ~~Add `[workspace.lints.clippy]` + `[lints] workspace = true` per crate~~ — done in S-001 (with group priority `-1` and the three documented exceptions above).
  - ~~Pin the toolchain via `rust-toolchain.toml`~~ — done in S-001 (`1.99.1`, components cargo/clippy/rustfmt, target wasm32-unknown-unknown).
  - Add the `cargo clippy --workspace --all-targets` CI job once a CI provider exists (open item in [§6](../arc42/06-quality-model.md)).
  - Toolchain bumps: re-run the full gate and treat new nursery/restriction hits as deliberate change review, not noise.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v3 (2026-10-23, S-002):** seven more workspace-level exceptions documented with reasons, all found while writing the first real code (`diagram-model`): `missing_inline_in_public_items`, `question_mark_used`, `missing_trait_methods`, `pub_with_shorthand`, `exhaustive_structs`, `exhaustive_enums`, `pattern_type_mismatch`. The lint policy itself is unchanged (all groups deny); these are the minimal set of allows that real code needs. Verified: clean gate on `diagram-model` with 100% line coverage.
- **v2 (2026-10-22, S-001):** implemented and accepted. Mechanism refined to `{ level = "deny", priority = -1 }` per group (Clippy ≥ 1.93 makes same-priority group/individual lint mixes a hard error), and three workspace-level exceptions documented with reasons: `cargo_common_metadata`, `blanket_clippy_restriction_lints`, `implicit_return`. Verified: clean gate on the scaffold, negative probe confirms a lint violation fails the build.
- **v1 (2026-10-22):** initial decision — all Clippy lint groups at deny in every workspace crate; build fails on any violation; toolchain pinning as the nursery-volatility mitigation.
