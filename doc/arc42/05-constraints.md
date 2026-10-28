---
title: "arc42-5-constraints"
description: hard constraints — legal, boundary conditions, technical framework conditions
scope: architecture
status: draft
updated: 2026-10-23
---

# 5. Constraints

> *All hard constraints, grouped by category.*

## 5.1 Legal & Regulatory Constraints

- **Open-source product.** ascii-diagram-drawer is a community tool, not a commercial product (§3). The codebase ships under **Apache-2.0** ([ADR 015](../adr/015-apache-2-0-license.md); full canonical text in `LICENSE` at repo root); all dependencies in the distributed product must be permissively licensed. No GPL/AGPL components in core or GUI.
- **No font/glyph data licensing burden in v1.** Text output uses the user's system monospace font; SVG output is vector paths produced by `svgbob` (no embedded fonts). If a later version embeds a font (e.g. for standalone PNG rasterization), it must be OFL-licensed.
- **No regulatory constraints.** The tool handles no personal data, no industry-regulated content, and makes no network calls — nothing to comply with beyond licensing.

## 5.2 Boundary Conditions

- **Platform targets:** Windows, macOS, Linux (native desktop) **and** modern browsers via WebAssembly — from one codebase ([NFR1](14-important-requirements.md), Q2). No mobile target in v1 or v1+.
- **I/O formats:** input = `.ad` diagram files + shape template/collection files ([ADR 004](../adr/004-diagram-file-format.md), [ADR 007](../adr/007-user-defined-diagram-objects.md)); ASCII import (v1+, FR12). Output = plain text (UTF-8, LF), SVG, PNG (FR5).
- **Fully offline.** No network access, no telemetry, no external services — the tool must work with the network cable unplugged. This is a boundary condition by design, not an accident.
- **Monospace grid as the rendering domain.** All geometry lives on a character cell grid; text output must render correctly under *any* monospace font (only svgbob-alphabet glyphs + user labels are used — [ADR 001](../adr/001-glyph-resolution-alphabet.md)).
- **Scale boundary (proposed, pending confirmation):** diagrams up to ~500 objects / ~200×100 cells must stay fully interactive; beyond that, graceful degradation is acceptable.

## 5.3 Technical Framework Conditions

| Condition | Mandate / source |
|---|---|
| Language | Rust — workspace with platform-independent core crates shared by all GUI targets (AGENTS.md convention) |
| Core independence | Rendering/export/format logic: plain data in → text/SVG/PNG out, **no windowing or event-loop dependencies** in core; unit-testable headlessly (AGENTS.md, Q5/NFR5) |
| Vector rendering | `svgbob` crate interprets the resolved grid — no second vector path ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)) |
| GUI framework | Must target native desktop (Win/macOS/Linux) **and** web/WASM from one codebase (NFR1/Q2). **Chosen: egui / eframe** — [ADR 008](../adr/008-gui-framework-choice.md) (status `proposed`; Slint recorded as fallback) |
| Build tooling | `cargo`; web target `wasm32-unknown-unknown`; frontend bundling follows the GUI framework choice |
| Quality gates | `cargo fmt`, **Clippy's default group (`clippy::all`) active at deny — violations fail the build** ([ADR 010](../adr/010-deny-all-clippy-lints.md)), `cargo tarpaulin` ≥80% line coverage on core crates (AGENTS.md) |
| Performance budgets (proposed, pending confirmation) | First render < 100 ms for a 500-object diagram; interactive drag/resize feedback ≤ 16 ms per frame on desktop; WASM initial download < 2 MB compressed |
