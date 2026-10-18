---
title: 001 - unicode-glyph-matching-renderer
description: rendering uses Unicode box-drawing characters; the renderer resolves the matching glyph per cell wherever possible
scope: rendering
status: superseded by 006
updated: 2026-10-18
supersedes: null
---

# ADR 001: Unicode glyph-matching renderer

> **⚠️ Superseded by [ADR 006](006-svgbob-compatible-ascii-alphabet.md) (2026-10-18).** The Unicode box-drawing alphabet was replaced by svgbob's ASCII input alphabet as the canonical grid output. The glyph-resolution *mechanism* (one character per cell, resolved from all edges/content through it) is retained — only the target alphabet changed. Kept for history.

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
- **Follow-ups:** overlap/collision precedence for non-edge content (text vs. borders) is still open — tracked in concept C1, [glyph resolution](../arc42/12-concepts/glyph-resolution.md) ("How it works / behaves", fallback rules [TBD])
