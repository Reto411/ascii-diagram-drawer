---
title: "arc42-12-concepts"
description: index of architecture concepts (C1+) — each concept has a full doc in doc/arc42/12-concepts/
scope: architecture
status: draft
updated: 2026-10-18
---

# 12. Concepts

> *Important concepts with stable identities — the vocabulary of the architecture (e.g. "DiagramModel", "Cell grid", "Box-drawing renderer").*
>
> *This chapter is the **index**. Full concept docs live in this folder (`doc/arc42/12-concepts/`, one file per concept, created from `doc/templates/concept.md`).*

## Concept C1: Glyph Resolution

- **ID:** C1
- **Purpose:** the renderer assigns one matching Unicode box-drawing character per occupied grid cell, wherever possible (corners, T-junctions, crossings)
- **Details:** [`doc/arc42/12-concepts/glyph-resolution.md`](glyph-resolution.md) · decided in [ADR 001](../../adr/001-unicode-glyph-matching-renderer.md)
- **Interfaces (provided/required):** [TBD]
- **Dependencies:** [TBD]

*(Repeat per concept.)*
