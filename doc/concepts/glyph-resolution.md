---
title: glyph-resolution
description: how the renderer picks one Unicode box-drawing character per grid cell when multiple edges meet — read before working on text rendering
scope: rendering
status: draft
updated: 2026-10-17
---

# Concept: Glyph Resolution

## Definition

The process by which the renderer assigns exactly **one** Unicode box-drawing character to each occupied grid cell, based on all edges and content passing through that cell.

## Why it exists

Characters occupy single cells; where shape borders meet or cross (corners, T-junctions, crossings), several logical edges share one cell. Without resolution, joins would render as broken or overlapping glyphs. Decided in [ADR 001](../adr/001-unicode-glyph-matching-renderer.md).

## How it works / behaves

- Input: per-cell set of connections (N/E/S/W edge presence) + any non-edge content
- Output: the matching glyph, e.g. no connection end → `─`/`│`, corner → `┌ ┐ └ ┘`, three-way → `├ ┤ ┬ ┴`, four-way → `┼`
- The renderer always searches for a matching character **wherever possible** (ADR 001)
- Fallback rules for ambiguous cells (no single glyph fits, e.g. text under an edge): **[TBD — open]**

## Relations

- **Cell grid** — the coordinate system glyph resolution operates on (`doc/concepts/cell-grid.md`, not yet written)
- **Diagram model** — shapes declare edges; they never choose glyphs themselves

## Implementation

Not yet implemented. Expected home: platform-independent rendering crate (see AGENTS.md conventions).
