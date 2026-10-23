---
title: story-board
description: persistent story board — backlog / in-progress / blocked / done; the single source of truth for what to work on next
scope: delivery
status: stable
updated: 2026-10-23
---

# Story Board

## Backlog

| ID | Title | Priority | Dependencies | Acceptance criteria | Story file |
|----|-------|----------|--------------|---------------------|------------|
| S-005 | shape-registry — built-in catalog + template/collection loading | High | S-002, S-003 | 6 | [S-005](S-005-shape-registry-catalog.md) |
| S-006 | render-pipeline — place → resolve → ResolvedGrid (shapes + text) | High | S-002, S-005 | 7 | [S-006](S-006-render-pipeline-grid.md) |
| S-007 | render-pipeline — anchor-bound connectors (+ ADR 013 routing) | High | S-006, S-005 | 5 | [S-007](S-007-render-pipeline-connectors.md) |
| S-008 | exporters — text + SVG via svgbob | High | S-006, S-007 | 6 | [S-008](S-008-exporters-text-svg.md) |
| S-009 | exporters — PNG rasterization (+ ADR 014 rasterizer) | Medium | S-008 | 6 | [S-009](S-009-exporters-png.md) |
| S-010 | ui-api + core-services — service seam (ADR 009 pinned) | High | S-004, S-005, S-006, S-007, S-008, S-009 | 5 | [S-010](S-010-ui-api-core-services.md) |
| S-011 | core-services — responsive PNG worker (ADR 012) | Medium | S-010, S-009 | 6 | [S-011](S-011-png-worker-thread.md) |
| S-012 | gui-app — eframe scaffold + WYSIWYG canvas spike (ADR 008 milestone) | High | S-010, S-006 | 6 | [S-012](S-012-gui-canvas-spike.md) |
| S-013 | gui-app — editing gestures (move / resize / connect / select) | High | S-012, S-007 | 5 | [S-013](S-013-gui-editing-gestures.md) |
| S-014 | gui-app — file open/save + text output UX (FR2/FR4, RS2/RS5) | High | S-012, S-010 | 5 | [S-014](S-014-gui-file-io-text-output.md) |
| S-015 | gui-app — SVG/PNG export + scale control (FR5/FR8, RS4) | Medium | S-012, S-011 | 4 | [S-015](S-015-gui-export-scale.md) |
| S-016 | gui-app — collections sidebar + custom shape placement (FR3) | High | S-012, S-005 | 4 | [S-016](S-016-gui-collections-sidebar.md) |
| S-017 | gui-app — template editor: text art + graphical anchors (ADR 007 last piece) | High | S-016, S-005 | 5 | [S-017](S-017-gui-template-editor.md) |
| S-018 | gui-app — free text elements (FR10) | Medium | S-013, S-006 | 4 | [S-018](S-018-gui-free-text.md) |
| S-019 | web/wasm milestone — bundle build + core tests on wasm32 (QS4) | High | S-012…S-018, S-011 | 5 | [S-019](S-019-wasm-web-milestone.md) |
| S-020 | performance benchmarks + determinism invariants (QS1/QS12) | Low | S-008, S-011 | 4 | [S-020](S-020-performance-benchmarks.md) |
| S-021 | documentation completion (§4/§7/§10/§11/§13/§15/§16 + tracker) | Medium | S-019, S-020 | 4 | [S-021](S-021-docs-completion.md) |

## In progress

| ID | Title | Started | Notes |
|----|-------|---------|-------|
| — | — | — | — |

## Blocked

| ID | Title | Blocked on | Note |
|----|-------|------------|------|
| — | — | — | — |

## Done

| ID | Title | Completed | Verification summary |
|----|-------|-----------|----------------------|
| S-001 | Workspace scaffold + quality tooling | 2026-10-22 | all 6 ACs verified incl. negative probes (lint violation fails build; forbidden edge fails T5 audit); ADR 010 → accepted v2, ADR 011 → accepted; coverage baseline 0/1, 80% floor enforced from S-002 |
| S-002 | diagram-model — Diagram aggregate + cell geometry | 2026-10-23 | all 6 ACs verified: leaf crate (empty `[dependencies]`), 30/30 tests, clippy/fmt clean, tarpaulin (Llvm) 175/175 = 100%; concept C5 `cell-grid.md` created + C4 Implementation updated; §8.4.1 invariant split fixed (anchor-geometry check is render-time — needs the catalog); ADR 010 → v3 (7 new documented exceptions) |
| S-003 | .ad format family specification (docs) | 2026-10-23 | all 5 ACs verified: normative grammar in concept docs C6 `diagram-file-format.md` + C7 `shape-template.md` + C8 `collection.md` (EBNF, field-mapping table = every model field exactly once → QS7); escaping (`\" \\ \\n`), versioning (major reject / minor skip-and-preserve-verbatim), canonical serialization, edge cases all settled; ADR 007 anchor-marker question settled by coordinate-based `anchor at x y` declarations (no marker char → no svgbob collision); ADR 004 → v2, ADR 007 → v4 with spec links (both stay `proposed` until implemented); index + mkdocs nav wired; `mkdocs build --strict` exits 0 |
| S-004 | ad-format — parser + canonical serializer | 2026-10-23 | all 6 ACs verified: `parse_document` assembles through the `Diagram` aggregate (all invariants apply) with 1-based line/column on every error; canonical `serialize` byte-stable (golden tests `sample.ad`/`escapes.ad`); proptest lossless round-trip (QS7) passes; versioning: major reject, higher-minor skip-and-preserve-verbatim at original position (re-save keeps the source header so preserved statements stay legal on same-version re-parse); 60 crate tests + full workspace green under ADR 010 v4; tarpaulin (Llvm) 86.36% ≥ 80%; ADR 004 → accepted v3, concept C6 Implementation section updated, one new documented parse leniency (whitespace around the header dot) |

## Deferred (not v1)

- **FR12 / `ascii-import` context** — ASCII import is explicitly out of v1 scope (§14). The crate is scaffolded in S-001 as a documented stub so the T5 DAG audit can pin its edges; no functional story until v1+.
- **Custom shape resize rules** — ADR 007 v3: all user templates are fixed-size in v1; `resize_rules` exists on the type for later.
- **P1/P2/P3 budget confirmation** — budgets stay "proposed, awaiting confirmation" (§6); S-020 measures against them without gating.
- **CI provider selection** — open item from §8.5; gates run locally per the quality section until a provider is chosen (would be a new ADR).

## Progress log

- 2026-10-22: board created. Breakdown of arc42 + ADRs into 21 stories (S-001…S-021) in dependency order: scaffold → headless core bottom-up (diagram-model → ad-format spec+parser, shape-registry → render-pipeline → exporters) → service seam + PNG worker → GUI native (canvas spike → gestures → file IO → export/scale → collections → template editor → free text) → wasm milestone → benchmarks → docs completion. Proposed ADRs map to stories: 010→S-001, 011→S-001, 004→S-004, 001→S-006, 005→S-008, 009→S-010, 012→S-011, 008→S-012, 007→S-017. New ADRs to be created during work: 013 (connector routing, S-007), 014 (PNG rasterizer, S-009).
- 2026-10-22: **S-001 done.** Workspace (9 crates per §8.3) + all-groups-deny Clippy config (ADR 010 v2: group priority -1; exceptions `cargo_common_metadata`, `blanket_clippy_restriction_lints`, `implicit_return`) + toolchain pin 1.99.1 + T5 dependency-audit test in core-services. Gotcha recorded: tarpaulin needs `--engine Llvm` in this container (seccomp blocks the `personality` syscall); coverage baseline 0/1 — 80% floor enforced from S-002. Next: S-002.
- 2026-10-23: **S-003 done.** Format family specified as three concept docs (C6/C7/C8): `.ad` diagram grammar (statements `scale|shape|text|connect`, `as <id>` stable ids, `"…"` strings with `\" \\ \\n` escapes, `#` comments, EBNF + field-mapping table covering every `diagram-model` field exactly once), `.adc` collection syntax (file = manifest; identity = filename stem; bare type-id → built-ins only, `collection/name` → user collections, no shadowing), and template blocks (`template <name> <w>x<h>` + h grid rows + coordinate-based `anchor <name> at <x> <y>` — settles ADR 007's open marker-character question: there is none). Canonical serialization pinned (byte-stable re-save per ADR 004); versioning: major reject, minor skip-and-preserve-verbatim. ADR 004 → v2, ADR 007 → v4 (follow-ups point at the spec; both stay `proposed` — implementation is S-004/S-005). Next: S-004 (parser + canonical serializer) and S-005 (shape registry) are unblocked.
- 2026-10-23: **S-002 done.** `diagram-model` implemented per §8.4.1: `Diagram` aggregate (ordered objects = z-order + scale), `CellPos`/`CellSize`/`AnchorRef` value objects, `Object::Shape|Text|Connector`, intent-revealing mutations with boundary invariants (`ModelError` as the whole failure surface). 30 tests, 100% line coverage. Two doc fixes landed with it: §8.4.1 invariant split (anchor-geometry check moved to render time — impossible in a leaf crate; consistent with §8.2.3) and `ShapeInstance.label` added to the documented model (ADR 004 requires it). ADR 010 → v3: first real code exposed 7 more minimal Clippy exceptions, all documented. Next: S-003 (.ad format spec — unblocks S-004/S-005).
- 2026-10-23: **ADR 010 → v4 (human decision, direct request).** Clippy policy reduced from all-groups-deny to `clippy::all`-only deny — default-group violations still fail the build; `pedantic`/`restriction`/`nursery`/`cargo` no longer enforced. Root `Cargo.toml` updated (ten workspace exceptions retired), AGENTS.md, arc42 §5/§6, overview tracker, mkdocs nav and this skill aligned in the same commit. Follow-up (optional): remove now-dead per-item `#[allow]`s on restriction/nursery lints. Note: the working tree carries uncommitted S-004 work-in-progress (`ad-format` parser/serializer/tests) that predates this entry — recover it before starting S-004.
- 2026-10-23: **S-004 done.** Recovered the WIP from the tree and finished it: fixed the property-test generator (valid ids/anchors, connectors only reference existing shapes), aligned one stale test with the QS7-preserving serializer behavior (re-save keeps the source header version so preserved unknown statements stay legal on same-version re-parse), and added targeted error-path tests to lift coverage from 77.7% to 86.36%. One spec doc fix: the parser's lenient header tokenization (`ad 1 . 0`) is now documented in concept C6 parsing rules. ADR 004 → accepted (v3). Next: S-005 (shape registry — built-in catalog + `.adc` collection loading).
