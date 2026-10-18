---
title: "arc42-14-important-requirements"
description: architecturally relevant subset of functional and non-functional requirements, traceable to quality goals
scope: architecture
status: draft
updated: 2026-10-18
---

# 14. Identification of Important Requirements

> *The subset of functional and non-functional requirements that the architecture must satisfy — traceable to quality goals.*
>
> *arc42 convention: this section is a **selection**, not the complete requirements catalog. The full set of requirements is managed in: [TBD — e.g. separate spec doc, issue tracker]. Only architecturally relevant requirements are listed here.*

## 14.1 Functional Requirements

| ID | Requirement | Source (feature/user) | Priority |
|----|-------------|------------------------|----------|
| FR1 | User can place, move, and resize predefined shapes (boxes, circles, …) on a canvas | §1.1 Obj. 1 | High |
| FR2 | Diagram is exported as plain monospace text (svgbob-compatible ASCII per [ADR 001](../adr/001-glyph-resolution-alphabet.md)), copy/paste-able anywhere: terminals, READMEs, chat, docs — and directly parseable by svgbob tooling | §1.1 Obj. 2 | High |
| FR3 | Users can define their own diagram objects entirely (custom geometry/glyphs beyond the predefined set) and store them in named collections for grouping and reuse across diagrams | §1.1 Obj. 3, customer requirement | High |
| FR4 | Diagrams are saved to and loaded from the open plain-text file format (round-trip: reload → identical model) | §1 Background (open format) | High |
| FR5 | Diagrams export as SVG (vector) and PNG (raster) in addition to text; vector output is derived from the resolved character grid via svgbob ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)), so exports always match the text representation exactly | §1.1 Obj. 5, rendering model: svgbob (§1 Background) | Medium |
| FR6 | UI consists of a template sidebar + editor canvas (minimal) | §1.1 Obj. 6 | High |
| FR7 | Shapes align to the cell grid | §1.1 Obj. 7 | High |
| FR8 | A scale control scales the entire diagram's rendered output up (larger spacing → more readable text output), not just a display zoom | §1.1 Obj. 7 | Medium |
| FR9 | Connectors (lines/arrows) can be drawn between/around shapes; they bind to shape anchors and stay attached when the bound shape moves or resizes | Usage reference: asciilogic (§1 Background) | Medium |
| FR10 | Free text can be placed as an element on the canvas (independent of shapes) | Usage reference: asciilogic (§1 Background) | Low |

## 14.2 Non-functional Requirements

| ID | Requirement | Quality goal ref | Priority |
|----|-------------|------------------|----------|
| NFR1 | One codebase runs on Windows, macOS, Linux **and** in the browser (WASM) | [Q2](01-introduction-and-context.md#12-quality-goals-top-level) portability | High |
| NFR2 | File format is open and human-readable **without any renderer** (plain text) | [Q3](01-introduction-and-context.md#12-quality-goals-top-level) openness/readability | High |
| NFR3 | No automatic layout rearrangement — element positions are exactly where the user placed them; rendering is a **pure deterministic function** of the diagram model (no randomness, no external state); same model → byte-identical output | [Q1](01-introduction-and-context.md#12-quality-goals-top-level) determinism | High |
| NFR4 | Low learning curve / minimal UI complexity: a new user can compose a first diagram without documentation | [Q4](01-introduction-and-context.md#12-quality-goals-top-level) usability | Medium |
| NFR5 | Rendering/export core is platform-independent (shared crate) and unit-testable headlessly, with ≥80% coverage | [Q5](01-introduction-and-context.md#12-quality-goals-top-level) testability | Medium |

> *Notes: FR3's user-defined objects and collections are specified by [ADR 007](../adr/007-user-defined-diagram-objects.md) (proposed). NFR3 is the architectural answer to the PlantUML "spaghetti" problem (see §1 Background). Testable properties — NFR2: opening the file in any text editor shows the diagram; NFR3: rendering the same model twice yields byte-identical text. Quality goals defined in [§1.2](01-introduction-and-context.md#12-quality-goals-top-level); measurable scenarios pending in [§6](06-quality-model.md). FR4's file format: see [ADR 004](../adr/004-diagram-file-format.md) (proposed). FR9/FR10 derive from the asciilogic usage reference (§1 Background); FR9's connector statements feed ADR 004's open spec points. NFR3 + ADR 001 together make rendering a deterministic glyph lookup — no layout engine, no randomness. FR5: SVG/PNG share the resolved character grid with the text output — one deterministic pipeline (see concept C2, [render-pipeline](12-concepts/render-pipeline.md)); vector interpretation is delegated to svgbob directly on the canonical grid (ADR 005, ADR 001); the grid is a derived intermediate, not the storage format (ADR 004).*
