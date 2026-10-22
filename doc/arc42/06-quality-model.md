---
title: "arc42-6-quality-model"
description: complete quality attribute set — quality tree, measurable quality scenarios (usage + change), and the test strategy/coverage gates that verify them
scope: architecture / quality assurance
status: draft
updated: 2026-10-22
---

# 6. Quality Model

> *The complete set of quality attributes for this system and how they are measured/verified.*
>
> Top-level goals Q1–Q5 are defined in [§1.2](01-introduction-and-context.md#12-quality-goals-top-level) (official arc42 §01 role: 3–5 prioritized goals + one-line scenarios). This chapter is the detailed, measurable catalog — the role of official arc42 section 10 "Quality Requirements". Scenarios use the official short form (*situation/stimulus → response with measurable metric*) and cover **usage** *and* **change** scenarios. ISO 25010:2023 mapping: Q1 → reliability, Q2 → portability, Q3 → maintainability + compatibility, Q4 → usability (learnability), Q5 → maintainability (testability); performance budgets → performance efficiency.

## 6.1 Quality Tree

```
Q1 Deterministic output (H)
├── Q1.1 Same model → byte-identical text on any platform, at any time (0 diff bytes)
├── Q1.2 Text, SVG, PNG all derive from the ONE resolved character grid — exports are mutually consistent by construction
└── Q1.3 Rendering is a pure function: no dependence on wall-clock time, locale, threads, or environment

Q2 Cross-platform portability (H)
├── Q2.1 Same codebase builds and passes the full core test suite unmodified on Windows, macOS, Linux, and wasm32
└── Q2.2 Zero target-specific forks in core — platform adaptation lives only in GUI/app crates

Q3 Open, readable format (H)
├── Q3.1 Any .ad file is plain UTF-8 text: every shape, label, position, anchor visible without a renderer; no binary/base64 blobs
├── Q3.2 Lossless round-trip: load(save(m)) ≡ m (0 semantic diff)
└── Q3.3 Versioned format: new optional fields stay loadable by older versions within one major version; breaking changes require a version bump + migration

Q4 Low learning curve (M)
├── Q4.1 A first-time user places shapes, connects them, and exports — without reading the docs
└── Q4.2 v1 UI surface stays minimal: template sidebar + canvas only (FR6); new top-level UI needs an ADR

Q5 Testable core (M)
├── Q5.1 ≥80% line coverage on every core crate, enforced in CI (tarpaulin fail-under)
├── Q5.2 The entire render/export pipeline runs headlessly — no windowing/event-loop dependency anywhere in core
└── Q5.3 GUI layer stays thin: no rendering/format logic outside core

Performance (from §5.3 budgets — proposed, pending confirmation)
├── P1 First render of a 500-object / 200×100-cell diagram < 100 ms
├── P2 Interactive drag/resize feedback ≤ 16 ms per frame on desktop reference hardware
└── P3 Web initial download < 2 MB compressed
```

## 6.2 Quality Scenarios

| ID | Goal | Scenario (situation/stimulus → measurable requirement) | Verification |
|----|------|--------------------------------------------------------|--------------|
| QS1 | Q1.1 | Rendering any saved diagram model twice — on any supported platform, at any time — yields **byte-identical** text output (0 diff bytes) | Golden-file tests run on **every** CI platform; double-render assertion in unit tests |
| QS2 | Q1.2 | For any model, text, SVG, and PNG exports are derived from one resolved grid: a change to the model changes all three outputs consistently (no export can diverge from the text) | Integration test: fixture rendered through all three exporters; snapshot comparison of the shared grid stage |
| QS3 | Q1.3 | Rendering with different wall-clock times, locales, thread counts, and environment variables yields identical output (pure function) | Unit tests varying locale/time env where the platform allows; code review gate: no `Instant`/env/locale access in core (clippy lint list) |
| QS4 | Q2.1 | CI on Windows, macOS, Linux runners **and** the wasm32 target passes 100% of the core test suite unmodified | CI matrix job per target; wasm tests via `wasm-bindgen-test` |
| QS5 | Q2.2 | Adding a new platform target requires **zero** changes to core crates (adaptation only in app/GUI crates) | Architecture test: core compiles for all targets with an allow-listed set of `cfg` gates; review gate on any new `cfg(target_*)` in core |
| QS6 | Q3.1 | Opening any .ad file in a plain text editor shows every shape, label, position, and anchor as human-readable text — no binary or base64 sections | Format lint test: parser rejects/flags non-UTF-8 or blob payloads; fixture review in spec PRs |
| QS7 | Q3.2 | Saving then loading any diagram (including generated random models) yields a semantically identical model — 0 semantic diff | Property-based round-trip test (`proptest`): generate → save → load → assert equality |
| QS8 | Q3.3 | A file using new optional fields loads without data loss in the previous minor version; incompatible changes ship with a format version bump and a migration that passes fixture tests | Versioned fixture tests per format version; loader tests for unknown-field tolerance |
| QS9 | Q4.1 | In a usability study, ≥ 5 of 5 first-time users complete place → connect → export-text **without documentation** within 10 minutes | Manual usability study before v1 release (scripted task, recorded); result logged in release notes |
| QS10 | Q1.1 (change) | Adding a predefined shape type or glyph rule must not change the output of any existing diagram — golden files stay byte-identical unless explicitly re-blessed | Golden files pin outputs; CI fails on any unexplained golden diff; re-blessing is an explicit, reviewable commit |
| QS11 | Q5.1 (change) | Any change to a core crate merges only if line coverage stays **≥ 80%** for that crate | `cargo tarpaulin --fail-under 80` in CI per core crate |
| QS12 | P1/P2 | Rendering a 500-object / 200×100-cell diagram completes first render in **< 100 ms**; drag/resize feedback stays **≤ 16 ms/frame** on desktop reference hardware | `criterion` benchmarks in CI with regression thresholds (fail on > 20% slowdown) |
| QS13 | P3 | The web build's initial download is **< 2 MB compressed** | CI job measuring the wasm+JS bundle size with a budget gate |

> *Budgets referenced by QS12/QS13 come from [§5.3](05-constraints.md) and are still marked **proposed, pending confirmation** — the numbers move there first.*

## 6.3 Test Goals & Verification Strategy

The test strategy exists to make Q1–Q5 *continuously verified*, not spot-checked:

| # | Test goal | How |
|---|-----------|-----|
| T1 | **Determinism is a CI invariant, not a one-time check** — cross-platform byte-equality of the output is proven on every commit | Golden-file fixtures (text + SVG) rendered on all four CI targets; any drift fails the build (QS1, QS4, QS10) |
| T2 | **Lossless format by construction** — round-trip and forward-compatibility are property-tested, not example-tested | `proptest` generators for models: save/load equality, unknown-field tolerance (QS6–QS8) |
| T3 | **Alphabet purity** — rendered text contains only the canonical svgbob alphabet + user labels; no stray glyphs can reach output | Property test over generated diagrams asserting every cell is in the allowed set ([ADR 001](../adr/001-glyph-resolution-alphabet.md)) |
| T4 | **Coverage gate** — ≥80% line coverage on all core crates (model, format, render pipeline, exporters) before merge; GUI/app crate: thin by design (Q5.3), best-effort until the first milestone proves a stable baseline | `cargo tarpaulin --fail-under 80` per core crate in CI (QS11) |
| T5 | **Headless core, provably** — no windowing/event-loop dependency enters core; the full pipeline runs in plain unit tests | Dependency audit (core Cargo.toml allow-list) + Q5.2 architecture test (QS4) |
| T6 | **Performance budgets are regression-tested** — first-render and per-frame costs tracked, not vibes | `criterion` benchmarks with thresholds tied to §5.3 budgets (QS12); bundle-size gate for web (QS13) |
| T7 | **GUI stays verifiable despite limited egui test tooling** — the GUI layer is thin enough that behavior tests target the core it calls, and GUI logic runs against in-memory fakes of the `ui-api` traits ([ADR 009](../adr/009-ui-api-service-layer.md)); per-target manual smoke checklist (open, place, drag, connect, export × 4 targets) ships with each release | Thin-layer review gate + fake-backend GUI tests + release smoke checklist (Q5.3, QS9) |

**Definition of done for any change touching core:** `cargo fmt` + `cargo clippy` clean (**all lints deny** — [ADR 010](../adr/010-deny-all-clippy-lints.md)) · full test suite green on the local target · coverage ≥ 80% · no unexplained golden diff · docs updated per AGENTS.md.

> *Open items: (1) confirm §5.3 performance budgets to activate QS12/QS13 gates; (2) pick the CI provider/matrix once the workspace exists; (3) baseline GUI-crate coverage after the first eframe milestone — then decide whether to gate it.*
