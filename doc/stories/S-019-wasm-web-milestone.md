---
title: S-019-wasm-web-milestone
description: full web target — wasm bundle of gui-app builds + core test suites run on wasm32 (QS4); bundle size measured against P3
scope: workspace/gui-app
status: backlog
updated: 2026-10-22
---

# S-019: web/wasm milestone — build + tests on wasm

## Why

NFR1/QS2: one codebase, native + browser; the *same* test suite must pass on both targets (QS4). P3 (bundle < 2 MB) is a proposed budget — measure and report here. This closes the "browser" half of ADR 008's milestone matrix.

## Dependencies

- S-012…S-018 — all GUI functionality present (web build should not be feature-reduced; cfg-gated fallbacks only where a platform lacks an API)
- S-011 — wasm inline PNG path compiled in

## Acceptance criteria

1. **AC1 — Web bundle builds.** A documented command produces a servable web bundle of `gui-app` for wasm32-unknown-unknown (tooling choice — eframe web feature + wasm-bindgen/wasm-pack or trunk — recorded in Implementation notes); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0. Check: build command runs to completion; artifact exists.
2. **AC2 — Core tests on wasm.** The headless crates' test suites (`diagram-model`, `ad-format`, `shape-registry`, `render-pipeline`, `exporters`) run on wasm32 via a documented harness (e.g. wasm-bindgen-test) and pass; goldens identical to native runs. Check: harness command + pass output recorded in Verification.
3. **AC3 — No feature reduction.** The web app exposes the same features as native; platform-specific fallbacks (file IO, clipboard, PNG inline) are cfg-gated and listed in Implementation notes with a pointer to §10 deployment view. Check: code review checklist in Verification.
4. **AC4 — Bundle size measured.** Final JS+wat/wasm size recorded against the 2 MB proposed budget (P3); if over, note it as an open risk in `doc/arc42/15-risks.md` (budget stays "awaiting confirmation" — not a gate). Check: number recorded.
5. **AC5 — Gates green.** All standard gates still pass on the host target; overview tracker updated (Q2/NFR1 verified at build+test level; manual browser smoke checklist written for a human).

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
