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
| FR2 | Diagram is exported as plain monospace text (Unicode box-drawing per ADR 001), copy/paste-able anywhere: terminals, READMEs, chat, docs | §1.1 Obj. 2 | High |
| FR3 | Users can define custom shapes and group them into reusable collections across diagrams | §1.1 Obj. 3 | Medium |
| FR4 | Diagrams are saved to and loaded from the open plain-text file format (round-trip: reload → identical model) | §1 Background (open format) | High |
| FR5 | Diagrams export as SVG (vector) and PNG (raster) in addition to text | §1.1 Obj. 5 | Medium |
| FR6 | UI consists of a template sidebar + editor canvas (minimal) | §1.1 Obj. 6 | High |
| FR7 | Shapes align to the cell grid | §1.1 Obj. 7 | High |
| FR8 | A scale control scales the entire diagram's rendered output up (larger spacing → more readable text output), not just a display zoom | §1.1 Obj. 7 | Medium |

## 14.2 Non-functional Requirements

| ID | Requirement | Quality goal ref | Priority |
|----|-------------|------------------|----------|
| NFR1 | One codebase runs on Windows, macOS, Linux **and** in the browser (WASM) | [Q2](01-introduction-and-context.md#12-quality-goals-top-level) portability | High |
| NFR2 | File format is open and human-readable **without any renderer** (plain text) | [Q3](01-introduction-and-context.md#12-quality-goals-top-level) openness/readability | High |
| NFR3 | No automatic layout rearrangement — element positions are exactly where the user placed them; same model → reproducible (deterministic) output | [Q1](01-introduction-and-context.md#12-quality-goals-top-level) determinism | High |
| NFR4 | Low learning curve / minimal UI complexity: a new user can compose a first diagram without documentation | [Q4](01-introduction-and-context.md#12-quality-goals-top-level) usability | Medium |
| NFR5 | Rendering/export core is platform-independent (shared crate) and unit-testable headlessly, with ≥80% coverage | [Q5](01-introduction-and-context.md#12-quality-goals-top-level) testability | Medium |

> *Notes: NFR3 is the architectural answer to the PlantUML "spaghetti" problem (see §1 Background). Testable properties — NFR2: opening the file in any text editor shows the diagram; NFR3: rendering the same model twice yields byte-identical text. Quality goals defined in [§1.2](01-introduction-and-context.md#12-quality-goals-top-level); measurable scenarios pending in [§6](06-quality-model.md). FR4's file format: see [ADR 004](../adr/004-diagram-file-format.md) (proposed).*
