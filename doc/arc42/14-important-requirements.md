---
title: "arc42-14-important-requirements"
description: architecturally relevant subset of functional and non-functional requirements, traceable to quality goals
scope: architecture
status: draft
updated: 2026-10-17
---

# 14. Identification of Important Requirements

> *The subset of functional and non-functional requirements that the architecture must satisfy — traceable to quality goals.*
>
> *arc42 convention: this section is a **selection**, not the complete requirements catalog. The full set of requirements is managed in: [TBD — e.g. separate spec doc, issue tracker]. Only architecturally relevant requirements are listed here.*

## 14.1 Functional Requirements

| ID | Requirement | Source (feature/user) | Priority |
|----|-------------|------------------------|----------|
| FR1 | User can place, move, and resize predefined shapes on a canvas | §1.1 Obj. 1 | High |
| FR2 | Diagram is exported as plain monospace text (copy/paste anywhere) | §1.1 Obj. 2 | High |
| FR3 | Users can define custom shapes and group them into reusable collections | §1.1 Obj. 3 | Medium |
| FR4 | Diagrams export as SVG and PNG in addition to text | §1.1 Obj. 5 | Medium |
| FR5 | UI consists of a template sidebar + editor canvas (minimal) | §1.1 Obj. 6 | High |
| FR6 | Shapes align to a grid | §1.1 Obj. 7 | High |
| FR7 | A scale control (top-right) scales the entire diagram, improving ASCII readability | §1.1 Obj. 7 | Medium |

## 14.2 Non-functional Requirements

| ID | Requirement | Quality goal ref | Priority |
|----|-------------|------------------|----------|
| NFR1 | Runs on Windows, macOS, Linux **and** in the browser from one codebase | Q? (portability) | High |
| NFR2 | File format is open and human-readable **without any renderer** (plain text) | Q? (openness/readability) | High |
| NFR3 | No automatic layout rearrangement — element positions are exactly where the user placed them (deterministic output) | Q? (predictability) | High |
| NFR4 | Low learning curve / minimal UI complexity | Q? (usability) | Medium |

> *Notes: NFR3 is the architectural answer to the PlantUML "spaghetti" problem (see §1 Background). NFR2 testable property: opening the file in any text editor shows the diagram. Quality goal refs to be filled once §1.2/§6 are defined.*
