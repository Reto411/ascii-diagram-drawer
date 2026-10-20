---
title: "arc42-1-introduction-and-context"
description: project objectives (1.1), top-level quality goals, constraints, stakeholders — agents: always check 1.1 before work
scope: architecture
status: draft
updated: 2026-10-18
---

# 1. Introduction and Context

> *What is this project, what problem does it solve, and in which world does it live?*

## Background & Motivation

The target is a diagramming tool **similar to Visio**, but built on an **open source format**:

- Diagrams are stored in an open, plain-text format that can be **easily rendered** — and remains **human-readable even without a renderer** (any text editor or `cat` shows the diagram).
- Text-based diagram languages exist already (e.g. PlantUML), but they have a fundamental problem: in complex diagrams the **automatic layout engine rearranges the diagram by itself**, and the result turns into spaghetti.
- This tool prevents that with its **graphical overlay**: shapes are placed manually on a canvas (with grid alignment, see §1.1 objective 7), giving the user full control over where every element ends up — while still producing the plain-text output.
- **Usage-model reference:** [asciilogic](https://asciilogic.com) — canvas-based editing with tools for shapes, connectors, and text; connectors snap to shape anchors and stay bound when shapes move or resize. Our differentiators on top of that model: **fully user-defined diagram objects** and **collections** for grouping/reuse (Obj. 3).
- **Rendering model:** [svgbob](https://ivanceras.github.io/svgbob/) defines the grid→vector interpretation we use for SVG export — each glyph's shape depends on its neighboring cells, fragments merge into lines/arcs, and patterns are promoted to high-level shapes (rects, circles). We delegate that interpretation to the `svgbob` crate directly: our canonical grid uses svgbob's input alphabet, so the text output is exactly what `svgbob::to_svg` consumes ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md), [ADR 001](../adr/001-glyph-resolution-alphabet.md)) — **every output matches the text representation exactly**. Difference from svgbob: our character grid is *derived* from the object model at render time — it is not the storage format ([ADR 004](../adr/004-diagram-file-format.md)).

## 1.1 Objectives

> *Project goals in 3–7 bullet points. What must the finished product be able to do? Why does it exist?*

**Draft — pending confirmation:**

1. **Draw ASCII diagrams visually.** Users compose diagrams by placing, moving, and resizing predefined shapes (boxes, circles, …) on a canvas — instead of hand-typing box-drawing characters.
2. **Plain-text output.** The result is pure monospace text that can be copied and pasted anywhere: terminals, READMEs, chat, commit messages, docs.
3. **Custom diagram objects & collections.** Users can define their own diagram objects entirely — beyond the predefined set — and store them in named collections (groups) for reuse across diagrams.
4. **Cross-platform GUI.** One app that runs on Windows, macOS, Linux, and in the browser.
5. **Vector/raster export.** Unlike the previous project, diagrams can be rendered as SVG (vector) and PNG (raster) in addition to plain text.
6. **Low learning curve.** Minimal UI: a template sidebar to pick shapes from and an editor canvas to compose the diagram.
7. **Grid alignment & scaling.** Shapes align to a grid, and a scale button in the top-right corner scales the entire diagram up — mostly important for the ASCII representation (larger spacing = more readable text output).

## 1.2 Quality Goals (top-level)

> *The 3–5 most important quality attributes of this system (e.g. "fast rendering", "zero-dependency output", "works offline"). Detailed scenarios go in [§6](06-quality-model.md).*

| # | Quality goal | Concrete scenario (one line) | Priority (H/M/L) |
|---|--------------|------------------------------|------------------|
| Q1 | **Deterministic output** — rendering is a pure, deterministic function of the diagram model: no auto-layout, no randomness | Rendering the same model twice — on any platform, at any time — yields byte-identical text *and* SVG | H |
| Q2 | **Cross-platform portability** — one codebase runs on Windows, macOS, Linux, and in the browser (WASM) | The same codebase builds and passes the full test suite on all four targets without target-specific forks | H |
| Q3 | **Open, readable format** — diagram files are plain text: human-readable without a renderer, easy to diff in version control | `cat diagram.ad` shows every shape, label, and position; git diffs read as per-element changes | H |
| Q4 | **Low learning curve** — minimal UI; a new user can compose their first diagram quickly | A first-time user places shapes, connects them with arrows, and exports text — without reading the docs | M |
| Q5 | **Testable core** — rendering/export logic is platform-independent and unit-testable headlessly (no GUI) | The entire render/export pipeline runs in headless unit tests; core crates stay ≥80% line coverage | M |

> *Draft, derived from §1.1 objectives and Background (inherits their "pending confirmation" status). Detailed, measurable quality scenarios (usage **and** change scenarios, official arc42 scenario format: context/stimulus/metric) live in [§6](06-quality-model.md).*

## 1.3 Constraints

> *Hard limits that shape the design: tech stack, target platforms, performance budgets, licensing, offline requirements.* See [§5](05-constraints.md) for details.

- [TBD]

## 1.4 Stakeholders

> *Who cares about this project and why? (users, developers, operators).* Details in [§4](04-stakeholders-and-users.md).

| Stakeholder | Interest / concern |
|-------------|--------------------|
| [TBD] | |
