---
title: glyph-resolution
description: how the renderer picks one svgbob-alphabet character per grid cell when multiple edges meet — read before working on text rendering
scope: rendering
status: stable
updated: 2026-10-26
---

# Concept: Glyph Resolution

## Definition

The process by which the renderer assigns exactly **one** svgbob input-alphabet character ([ADR 001](../../adr/001-glyph-resolution-alphabet.md)) to each occupied grid cell, based on all edges and content passing through that cell.

## Why it exists

Characters occupy single cells; where shape borders meet or cross (corners, T-junctions, crossings), several logical edges share one cell. Without resolution, joins would render as broken or overlapping glyphs. Decided in [ADR 001](../../adr/001-glyph-resolution-alphabet.md): v1 fixed the mechanism (one character per cell, resolved from all edges/content passing through it), v2 fixed the target alphabet (svgbob's input set).

## How it works / behaves

### Cell state

After the place stage, every grid cell holds three things:

- an **edge set** — which of the four directions N/E/S/W carry a line through the cell;
- optional **content** — at most one non-edge character (shape art, label character, text);
- a **structural-top flag** — whether the *last* writer to the cell was structural (an edge) or content.

### Rule table (the 16 connection states)

Resolution maps the per-cell N/E/S/W connection set to exactly one glyph:

| Connections | Glyph | Class |
|---|---|---|
| — (no edges, no content) | ` ` | empty cell (trimmed away by resolve) |
| N+S | `|` | straight vertical |
| E+W | `-` | straight horizontal |
| N+E / N+W / S+E / S+W | `+` | corner |
| any three directions | `+` | T-junction |
| N+E+S+W | `+` | cross |
| N or S alone | `|` | dangling stub (connector endpoint, S-007) |
| E or W alone | `-` | dangling stub (connector endpoint, S-007) |

**Priority:** cross > T-junction > corner > straight. The table is evaluated top-down; in v1 every junction class maps to the sharp `+`, so the priority only becomes observable once a later version splits the classes (e.g. rounded corners). Dangling single directions keep their straight glyph — they are how connector endpoints will render in S-007.

**Rounded corners, dashes, arrows, ports** (`. , ' `` ` `, `~ _ : !`, `< > ^ v V`, `* o O`) are *not produced by the rule table in v1*. They appear only as **verbatim pattern art** — the built-in ellipse's oval (re-rasterized at the instance extent, B-001) or a custom template's glyphs ([shape template](shape-template.md), ADR 007) — stamped opaquely and never re-resolved.

### Fallback rules (ambiguous cells)

- **F1 — last writer wins (z-order).** A cell holding both edges and content shows whichever was stamped last: content stamped after the edge shows the character (text can overpaint a border); an edge stamped after the content shows the resolved edge glyph (a border drawn later covers text). Document order is z-order, so this is fully deterministic.
- **F2 — edges merge.** Multiple structural stamps on one cell OR-merge their direction bits; the merged set is then resolved once through the rule table (this is how two adjacent boxes share a wall and crossing borders form `+`).
- **F3 — labels clip deterministically.** A label centers in its region with a *floor* offset: `offset = floor((region − line) / 2)` per axis. Characters falling outside the region are dropped; on odd overflow one extra character is clipped from the left/top. No reflow, no ellipsis.
- **F4 — quote fallback.** A label line containing any svgbob-alphabet character is wrapped in double quotes so it reads as literal text (ADR 001 v2). If the quoted form does not fit the region whole, the *raw* line is stamped instead (deterministic degradation, no partial quoting).
- **F5 — spaces are transparent.** A space character never stamps: it neither sets content nor clears earlier content or edges. Pattern art and label lines are therefore safe to overlay on anything.

### Resolution order

Strict row-major over the placed grid; each cell is resolved independently (no cross-cell lookups), so the output can never depend on iteration order of any map. The resolved grid is trimmed to the bounding box of occupied cells before scale expansion.

## Relations

- **Cell grid** — the coordinate system glyph resolution operates on
- **Diagram model** — shapes declare edges; they never choose glyphs themselves
- **Shape template** — `Pattern` geometry supplies verbatim art and live anchor cells that bypass the rule table until a connector binds (S-007)
- **Render pipeline** — glyph resolution is stage 2 of the pipeline (C2)

## Implementation

Implemented in `crates/render-pipeline`:

- `resolve.rs` — `resolve_edges` (the rule table above, all 16 states) and `resolve_cell` (F1: structural-top vs content); unit test covers every table row.
- `place.rs` — cell-state stamping (edge merge F2, z-order F1, label centering/clipping F3, quoting F4, space transparency F5).
- `grid.rs` — `ResolvedGrid` + `expand` (scale stage, C2/FR8).
- Golden files: `crates/render-pipeline/tests/golden/*.grid` (one per pattern and fallback case).
