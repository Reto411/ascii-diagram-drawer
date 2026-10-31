---
title: resize-rules
description: how custom shape templates declare per-edge grow/shrink patterns so they can be resized — read before implementing template resizing (S-024) or touching re-rasterization
scope: format / domain model / rendering
status: draft
updated: 2026-10-31
---

# Concept: Resize Rules

## Definition

A **resize rule** is the declaration a [shape template](shape-template.md) carries that makes its instances resizable: for each of the four edges, which character(s) are inserted when the shape grows in that direction and removed when it shrinks — plus optional **parity variants** (alternate patterns selected by even/odd width or height) so curved shapes stay correct at every size.

## Why it exists

FR3 extended by CR-003 (accepted 2026-10-31): custom objects must be fully user-defined *and* usable — a fixed-size-only template is a visible limitation versus built-in shapes (ADR 007 v3 consequence). Re-rasterizing arbitrary user art from scratch is not feasible, so resizing works by **edge pattern substitution** on the authored grid: the interior stays exactly as drawn, only the edge rows/columns gain or lose repeated characters. ADR 007 point 4 records the decision; this doc defines the mechanism.

## How it works / behaves

- **Per-edge rules.** Each edge (top, bottom, left, right) declares an **insert** character (or short pattern) added per step of growth and a matching **remove** behavior when shrinking:
  - *Box example:* top/bottom insert `─` between the corner characters; left/right insert `│`. Growing by one column appends one `│` to every interior row and widens the top/bottom rows by one `─` before the right corner.
  - Corners are **fixed**: they are never inserted or removed — only the edge runs between them stretch. A rule set therefore implicitly treats the template's four corner cells as anchors of the pattern.
- **Parity variants.** A rule may declare alternate insert patterns for **even** and **odd** extents (per axis). This is how an ellipse-shaped template stays smooth: the horizontal top/bottom runs differ between even and odd widths, so each target size selects the matching variant instead of repeating one pattern blindly.
- **Re-rasterization is pure.** `rasterize(template, rules, target_w, target_h) → grid` is a deterministic function (NFR3): given the authored grid, its rules, and a target size, the result is fully determined — no search, no randomness. The pipeline calls it at render time for instances whose size differs from the template's natural size, exactly like built-in `Ellipse` re-rasterization (B-001); the stored instance size is what the user set in the GUI.
- **Bounds.** Shrinking below the template's minimum extent (the span of its corner cells + 1) is rejected — a shape can never resize past destroying its corners. Growing is unbounded within u32 cell sizes.
- **No rules → fixed size (unchanged).** Templates without resize rules keep today's behavior: resize is rejected, instances stay at natural size (ADR 007 point 4). Unknown imported objects remain fixed-size until registered with rules (FR12).
- **File syntax (`.adc`, minor version 1.1).** `resize` statements inside a template block declare the rules; exact statement grammar is finalized in S-024's acceptance criteria and lands in [shape-template](shape-template.md)'s file-syntax section in the same commit. Unknown-statement tolerance applies as usual, so older readers skip-and-preserve them.
- **GUI.** The template editor (S-017) gains a rule-authoring surface: per-edge insert/remove character fields + parity toggles, with a live preview at several sizes through the existing `Renderer::preview_pattern` seam path. Resizing a placed instance that has rules is enabled in the canvas gestures (S-013's resize handle applies to it).

## Relations

- **Shape Template (C7)** — rules are part of the template definition, stored alongside grid + anchors (`doc/arc42/12-concepts/shape-template.md`)
- **Collection (C8)** — `resize` statements live in `.adc` template blocks; versioning per the shared scheme (`doc/arc42/12-concepts/collection.md`)
- **Render Pipeline (C2)** — re-rasterization happens at the place stage, replacing the natural-size stamp for instances with non-natural sizes (`doc/arc42/12-concepts/render-pipeline.md`)
- **Cell Grid (C5)** — target sizes are `CellSize` value objects; instance extent stays a cell rectangle (`doc/arc42/12-concepts/cell-grid.md`)

## Implementation

Not yet implemented — S-024 (CR-003): `.adc` `resize` statements in `shape-registry`, `rasterize()` in `render-pipeline` (or a shared module beside the built-in `Ellipse` re-rasterizer), rule fields in the template editor, resize-gesture enablement in `gui-app`.
