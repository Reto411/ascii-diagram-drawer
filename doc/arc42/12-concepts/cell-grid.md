---
title: cell-grid
description: the 2D integer grid all diagram geometry lives on — CellPos/CellSize value objects, top-left origin, whole-diagram scale; read before touching geometry or rendering
scope: model
status: stable
updated: 2026-10-23
---

# Concept: Cell Grid

## Definition

The canvas is an unbounded 2D grid of **cells**; every object's position and size is expressed in integer cell units with a top-left origin. At scale 1, one cell holds exactly one character of the resolved output — the grid *is* the text layout.

## Why it exists

Grid alignment (FR7) and the WYSIWYG promise: because geometry is integral and shared by every object, rendering is a pure function `Diagram → character grid` with no floating-point layout, so text/SVG/PNG outputs are bit-identical across runs (Q1). It is also what makes the `.ad` format compact — objects store cell coordinates, not pixels ([ADR 004](../../adr/004-diagram-file-format.md)).

## How it works / behaves

- **Value objects:** `CellPos { x, y }` and `CellSize { width, height }`, all `u32` — negative coordinates and zero-sized objects are unrepresentable by construction (`CellSize::new` rejects 0 dimensions).
- **Origin & axes:** `(0, 0)` is the top-left cell; `x` grows right, `y` grows down. The grid is unbounded: nothing requires objects to start at or stay within a fixed canvas — the rendered extent is derived from the occupied cells.
- **Scale (FR8):** the `Diagram` carries one whole-diagram scale factor (always ≥ 1), applied at render time: each cell becomes a `scale × scale` block of characters (and the corresponding SVG/PNG area). Scale is a property of the diagram, never of individual objects.
- **Occupancy:** rendering walks objects in document order (z-order) and stamps their glyphs into the grid; later objects overwrite earlier cells. Text objects are stamped from their content (one line per row); shapes from their catalog template.

## Relations

- **Diagram aggregate** — owns the scale and holds objects whose geometry is cell-based (link: `doc/arc42/12-concepts/diagram-aggregate.md`)
- **Glyph resolution** — assigns characters to occupied cells of this grid (C1, link: `doc/arc42/12-concepts/glyph-resolution.md`)
- **Render pipeline** — consumes the aggregate and resolves it onto this grid (C2, link: `doc/arc42/12-concepts/render-pipeline.md`)

## Implementation

`crates/diagram-model/src/geometry.rs` — `CellPos`, `CellSize` (with construction-time validation). The scale factor lives on the aggregate root (`crates/diagram-model/src/lib.rs`, `Diagram::scale` / `Diagram::set_scale`). Occupancy stamping is stage 1 of the render pipeline (not yet implemented, S-007+).
