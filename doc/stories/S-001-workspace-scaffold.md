---
title: S-001 - workspace-scaffold
description: cargo workspace with all 9 crates per §8.3, ADR 010 lint config, toolchain pin, T5 dependency audit test
scope: docs/tooling
status: backlog
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

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
