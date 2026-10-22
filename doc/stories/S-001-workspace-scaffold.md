---
title: S-001 - workspace-scaffold
description: cargo workspace with all 9 crates per §8.3, ADR 010 lint config, toolchain pin, T5 dependency audit test
scope: docs/tooling
status: done
updated: 2026-10-22
---

# S-001: Workspace scaffold + quality tooling

## Why

Everything else depends on the crate layout of [§8.3](../arc42/08-building-block-view.md) existing with its dependency rules enforced ([ADR 011](../adr/011-domain-driven-design.md)), and on the quality gates (fmt/clippy/test/coverage) being green from day one ([ADR 010](../adr/010-deny-all-clippy-lints.md), [§6 NFR5](../arc42/06-quality-model.md)).

## Dependencies

- none

## Acceptance criteria

1. **AC1 — Workspace compiles.** Root `Cargo.toml` declares a workspace with exactly: `diagram-model`, `ad-format`, `shape-registry`, `render-pipeline`, `exporters`, `ascii-import` (v1+ stub, documented as such), `ui-api`, `core-services`, `gui-app`. Check: `cargo build --workspace` exits 0.
2. **AC2 — Clippy denies all lints workspace-wide.** `[workspace.lints.clippy]` sets every lint group to `deny`; every crate opts in via `[lints] workspace = true`. A deliberate lint violation (e.g. `let _x = 1;`) fails `cargo clippy --workspace --all-targets`. Check: run that probe, confirm failure, revert.
3. **AC3 — Toolchain pinned.** `rust-toolchain.toml` pins the exact stable toolchain in use. Check: file exists, `rustup show active-toolchain` matches.
4. **AC4 — T5 dependency audit test passes and catches violations.** A workspace-level integration test parses every crate's `Cargo.toml` and asserts the declared edges equal the allow-list from §8.3 (gui-app → ui-api + core-services only; no cycles; context crates never depend on `ui-api`/`core-services`/`gui-app`). Check: `cargo test -p <audit host>` passes; adding a forbidden edge to a scratch crate makes it fail (verify, then revert).
5. **AC5 — All gates green.** `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, `cargo tarpaulin --workspace` all pass (≥80% where code exists; empty crates exempt but reported).
6. **AC6 — ADR 010 flipped to accepted** (lint config + toolchain pin landed) and ADR 011 flipped to accepted (crate boundaries + context mapping materialized as crates + T5 audit); version history entries added where the status flips; `doc/arc42/overview.md` tracker updated.

## Implementation notes

- Workspace root `Cargo.toml`: 9 members per §8.3, resolver v2, shared `[workspace.package]` metadata (edition 2021, license MIT OR Apache-2.0), `[workspace.lints.clippy]` with all five groups at `{ level = "deny", priority = -1 }` plus the three documented exceptions of [ADR 010](../adr/010-deny-all-clippy-lints.md) v2 (`cargo_common_metadata`, `blanket_clippy_restriction_lints`, `implicit_return`).
- `rust-toolchain.toml`: channel `1.99.1` (exact stable in use), components cargo/clippy/rustfmt, target wasm32-unknown-unknown.
- All crates are doc-comment stubs naming their bounded context; `gui-app/src/main.rs` is an empty `fn main()` until S-012. `shape-registry` is a leaf (anchor geometry stays registry-local — no shared-kernel edge to `diagram-model`).
- T5 audit: `crates/core-services/tests/dependency_audit.rs` (hosted in core-services, the composition root of the DAG). Test 1 pins the member set; test 2 parses each manifest and asserts every *normal* dependency on a workspace crate is on the §8.3 allow-list (dev-dependencies excluded — no runtime coupling; this test's own `toml` dev-dep proves it).
- Container gotcha: tarpaulin's default ptrace engine hard-fails here (`personality` syscall blocked by seccomp → "ASLR disable failed: ENOSYS"; 0.37.5 has no config escape hatch). Gate command in this environment: `cargo tarpaulin --engine Llvm --workspace`. Recorded as a convention in AGENTS.md.
- Coverage baseline at scaffold: 0/1 lines (the single countable line is the empty `main()` stub; all other crates are doc comments only). The 80% floor is enforced from S-002, the first story with substantive code.

## Verification

- [x] AC1 — `cargo build --workspace` exits 0 (9 members, exact set per §8.3).
- [x] AC2 — negative probe: appended `fn probe_violation() -> i32 { 1 + 0 }` to `diagram-model` → `cargo clippy -p diagram-model` failed with 3 lint errors (missing_docs, missing_const_for_fn, identity_add); reverted, gate green again.
- [x] AC3 — `rust-toolchain.toml` pins `1.99.1`; active toolchain resolves to it (`cargo 1.99.1`, rustc 1.99.1).
- [x] AC4 — audit passes (2/2). Negative probe: added forbidden edge `gui-app → ad-format` → test failed with "`gui-app` depends on workspace crate `ad-format`, which is not on the §8.3 allow-list"; reverted, green again. (First probe attempt exposed a real bug in the audit — it filtered dep keys by member *paths* instead of package *names*, making the check vacuous; fixed before accepting.)
- [x] AC5 — `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` clean; `cargo test --workspace` 18/18 test binaries ok (2 audit tests); `cargo tarpaulin --engine Llvm --workspace` runs green, coverage reported at 0/1 (baseline, see notes).
- [x] AC6 — ADR 010 → accepted v2 (mechanism + exceptions recorded), ADR 011 → accepted; C3 concept Implementation updated; `doc/arc42/overview.md` tracker updated. 
