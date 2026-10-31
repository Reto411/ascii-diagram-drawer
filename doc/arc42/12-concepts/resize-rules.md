---
title: resize-rules
description: how custom shape templates declare per-edge grow/shrink patterns so their instances can be resized — read before touching re-rasterization, the .adc resize grammar, or template-editor rule UI
scope: format / domain model / rendering
status: stable
updated: 2026-10-31
---

# Concept: Resize Rules

## Definition

A **resize rule** is the declaration a [shape template](shape-template.md) carries that makes its instances resizable: for each of the four edges, which character is inserted when the shape grows in that direction and which (matching) character may be removed when it shrinks — plus optional **parity variants** (alternate insert/remove pairs selected by even/odd target extent) so curved shapes stay correct at every size.

## Why it exists

FR3 extended by CR-003 (accepted 2026-10-31): custom objects must be fully user-defined *and* usable — a fixed-size-only template is a visible limitation versus built-in shapes (ADR 007 v3 consequence). Re-rasterizing arbitrary user art from scratch is not feasible, so resizing works by **edge pattern substitution** on the authored grid: the interior stays exactly as drawn, only the edge rows/columns gain or lose repeated characters. ADR 007 point 4 records the decision; this doc defines the mechanism.

## How it works / behaves

- **Per-edge rules.** Each edge (top, bottom, left, right) declares an **insert** character and a matching **remove** character:
  - *Box example:* top/bottom insert/remove `-`; left/right insert/remove `|`. Growing by one column widens each row's run before its last cell; the interior rows are widened with their own fill (auto-derived, see below).
- **How width and height changes apply** (`rasterize`):
  - *Width w → W:* every row keeps its first and last cell fixed. The run just before the last cell is extended or truncated. The top row uses the **top** edge rule, the bottom row the **bottom** edge rule; interior rows use an **auto-derived fill** — the character of the cell just inside their right border (space for bordered shapes, the fill glyph for solid ones) — so left/right rules are *not* consulted for width changes.
  - *Height h → H:* whole rows are added or removed just above the bottom row. A new row is `left_char + fill × (W−2) + right_char` from the **left/right** edge rules, with the fill auto-derived from the art's second-to-last original row. A removed row must have its first cell equal to left's remove char, its last cell equal to right's remove char, and a uniform interior — otherwise the shrink is rejected.
  - *Degenerate art:* 1-row templates are their own top edge; 1-wide rows extend to the right of their single cell.
- **Parity variants.** An edge rule may declare `even`/`odd` overrides (each an insert/remove pair) that replace the base pair whenever the target extent has that parity. This is how a curved template stays smooth: each target size selects the matching variant instead of repeating one pattern blindly. Overrides always ride alongside a base pair for the same edge.
- **Re-rasterization is pure.** `rasterize(template, rules, width, height) → grid` is a deterministic function (NFR3): given the authored grid, its rules, and a target size, the result is fully determined — no search, no randomness. Identity: target equal to the natural size returns the template itself. The pipeline calls it at render time for instances whose size differs from the template's natural size, exactly like built-in `Ellipse` re-rasterization (B-001); the stored instance size is what the user set in the GUI.
- **Bounds — art preservation.** Growth never fails; shrink stops with an error when a step would remove a cell that is not the declared (or auto-derived) remove character, or cross the minimum extent. The **minimum size** (`min_size`) is the smallest reachable by repeated valid single-step shrinks per axis — computed by simulation, so it accounts for non-uniform rows, not just corner spans. It bounds resize gestures in the GUI and clamps render-time sizes; a shape can never resize past destroying its corners or art.
- **No rules → fixed size (unchanged).** Templates without resize rules keep today's behavior: resize is rejected, instances stay at natural size (ADR 007 point 4). Unknown imported objects remain fixed-size until registered with rules (FR12).
- **File syntax (`.adc`, minor version 1.1).** One statement per (edge, parity), inside the template block:

  ```ebnf
  resize-stmt = "resize", edge, [ "even" | "odd" ], "insert", quoted-char, "remove", quoted-char
  edge        = "top" | "bottom" | "left" | "right"
  ```

  A statement without a parity keyword declares the edge's base pair; `even`/`odd` statements declare overrides. All four **base** pairs are required to enable resizing — a partial set is a parse error; duplicates per (edge, parity) are rejected. Unknown-statement tolerance applies as usual, so older readers skip-and-preserve them.
- **GUI.** The template editor's rule section: per-edge insert/remove character fields + optional even/odd override fields, enabled by a checkbox; enabling requires all four base pairs and rejects out-of-range characters. The live preview shows the current size plus previews at several other sizes through the `Renderer::preview_resized` seam. Resizing a placed instance that has rules is enabled in the canvas gestures (the S-013 resize handle applies to it), clamped to `min_size`.

## Relations

- **Shape Template (C7)** — rules are part of the template definition, stored alongside grid + anchors (`doc/arc42/12-concepts/shape-template.md`)
- **Collection (C8)** — `resize` statements live in `.adc` template blocks; versioning per the shared scheme (`doc/arc42/12-concepts/collection.md`)
- **Render Pipeline (C2)** — re-rasterization happens at the place stage, replacing the natural-size stamp for instances with non-natural sizes (`doc/arc42/12-concepts/render-pipeline.md`)
- **Cell Grid (C5)** — target sizes are `CellSize` value objects; instance extent stays a cell rectangle (`doc/arc42/12-concepts/cell-grid.md`)

## Implementation

Landed in S-024 (CR-003):

- `crates/shape-registry/src/types.rs` — `ResizeRules { top, bottom, left, right: EdgeRule }`, `EdgeRule { insert, remove, even, odd: Option<ParityChars> }`, `chars_for(target_extent)` parity selection; `ShapeType::resize_rules`.
- `crates/shape-registry/src/resize.rs` — pure `rasterize()` (edge pattern substitution above) and `min_size()` (shrink simulation); `ResizeError { BelowMinimum, RuleMismatch, RowContainsArt }` in `error.rs`.
- `crates/shape-registry/src/templates.rs` / `serializer.rs` — `.adc` v1.1 `resize` statement parsing/emission; round-trip preserved.
- `crates/render-pipeline/src/place.rs` — instances with non-natural sizes are re-rasterized at the place stage (clamped to `min_size`; anchors remapped proportionally via `map_proportional`, same formula as before).
- `crates/ui-api/src/lib.rs` — `Renderer::preview_resized(rows, rules: Option<&ResizeRuleSpec>, width, height)` seam (default: error); `crates/core-services/src/renderer.rs` implements it by building the pattern and running the place stage.
- `gui-app` — `template_editor.rs` rule-authoring UI (fields, validation, live previews at several sizes); `canvas.rs` draws the resize handle for custom objects with rules; `editor.rs` consumes the drag and clamps to `min_size`; catalog surfaces `resizable`/`min_size` per shape.
