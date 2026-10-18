---
title: "arc42-16-glossary"
description: project-specific terms — one shared vocabulary for docs and code
scope: architecture
status: draft
updated: 2026-10-18
---

# 16. Glossary

> *Definitions of project-specific terms, so the documentation (and code) share one vocabulary.*

| Term | Definition |
|------|------------|
| **Glyph resolution** | Assigning exactly one Unicode box-drawing character to each occupied grid cell, based on all edges/content passing through it (concept C1 — [glyph-resolution](12-concepts/glyph-resolution.md)) |
| **Cell grid** | The monospace character grid a diagram is placed on and rendered into; shapes occupy cells (concept doc not yet written) |
| **Diagram object** | A placeable element on the canvas — a predefined or user-defined shape, connector, or text; user-defined objects are stored in collections ([§1.1](01-introduction-and-context.md#11-objectives) Obj. 3) |
| **Shape collection** | A named group of diagram object definitions that users can store and reuse across diagrams |
| **Render pipeline** | The deterministic path model → resolved character grid → text/SVG/PNG; vector output is derived from the same grid as the text (concept C2, [render-pipeline](12-concepts/render-pipeline.md)) |
| **Shape template** | A predefined, reusable shape definition users can place on the canvas (see [§1.1](01-introduction-and-context.md#11-objectives)) |
