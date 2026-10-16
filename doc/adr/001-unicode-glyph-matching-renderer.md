---
title: 001 - unicode-glyph-matching-renderer
description: rendering uses Unicode box-drawing characters; the renderer resolves the matching glyph per cell wherever possible
scope: rendering
status: accepted
updated: 2026-10-17
supersedes: null
---

# ADR 001: Unicode glyph-matching renderer

## Context

The project name says "ASCII", but the output is **Unicode**: box-drawing characters (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`, …). When shape edges meet or cross on the shared cell grid, one cell can belong to several edges — the renderer must decide which single character to display there.

## Decision

The renderer works on a cell grid and **searches for a matching Unicode box-drawing character wherever possible**: each occupied cell is resolved to the glyph that fits all edges/content passing through it (e.g. corner → `┌`, T-junction → `┬`/`├`, crossing → `┼`). Choosing what to display is the renderer's responsibility, not the shapes'.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Pure ASCII (`+ - \|`) | Less readable; Unicode output is desired despite the project name |
| Static per-shape glyphs (no junction resolution) | Corners/junctions break where shapes meet or touch |

## Consequences

- **Positive:** clean joins between shapes, much more readable diagrams
- **Negative / costs:** need a glyph-resolution table/algorithm; ambiguous cells (e.g. 5+ connections, text under an edge) need defined fallback rules
- **Follow-ups:** overlap/collision precedence for non-edge content (text vs. borders) is still open — see arc42 §1.1 open points
