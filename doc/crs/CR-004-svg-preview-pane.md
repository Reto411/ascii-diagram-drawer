---
title: CR-004 - right-side SVG preview pane
description: a resizable, hideable right-side pane showing the diagram's effectively rendered SVG (IDE-style split view), with a "Preview" bookmark at the right edge to re-open it
scope: gui (preview pane) + export pipeline reuse
status: proposed
priority: medium
updated: 2026-10-30
---

# CR-004: Right-side SVG preview pane

## Requested change

A **preview pane on the right side of the window** shall show the prerendered **SVG** of the current diagram, like a split view in an IDE. The pane is **resizable by dragging its border** (width slide) and **hideable**; while hidden, a **"Preview" bookmark at the right edge of the window** re-opens it.

## Motivation

User feedback: they want to see the **effectively rendered diagram** — the exact SVG output that Export writes — while editing, not only the canvas's character-grid view. The text canvas and the vector interpretation can look different (e.g. curve/glyph rendering), so a live side-by-side preview closes that gap.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR6 | "UI consists of a template sidebar + editor canvas (minimal)" | "UI consists of a template sidebar + editor canvas, plus an optional right-side preview pane showing the diagram's rendered SVG — resizable by dragging its border, hideable, and re-opened via a 'Preview' bookmark at the right edge of the window (minimal)" |
| FR5 | "Diagrams export as SVG (vector) and PNG (raster) in addition to text; vector output is derived from the resolved character grid via svgbob ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)), so exports always match the text representation exactly" | unchanged — contextually affected: the preview shows exactly what FR5's SVG export produces (same pipeline, same bytes) |
| NFR3 | "…rendering is a **pure deterministic function** of the diagram model … same model → byte-identical output" | unchanged — constraint on the pane: preview content is the exact pipeline output for the current model (incl. FR8 scale), never an approximation |

## Conflict check

- **ADR 008 (egui/eframe):** egui has no native vector drawing — the pane displays the exact `to_svg` bytes **rasterized via resvg**, which is already a workspace dependency for PNG export ([ADR 016](../adr/016-png-rasterizer.md), S-009). Consistent, no new architectural decision; recorded as an assumption (what the user sees is byte-derived from the exported SVG, rasterized only for display).
- **Interaction with CR-002** (both reword FR6): order-independent — on acceptance, apply sequentially: whichever FR6 wording is current at that time gains the preview-pane sentence.
- No other recorded decision touched; no file-format impact.

## Impact analysis

- **Stories:** S-012 (done — eframe scaffold) rework: right-side panel + drag splitter + edge bookmark in the `app.rs` glue. S-008/S-009 (done — exporters): unchanged, reused as-is (`to_svg` bytes + resvg rasterization). S-015 (done — export UI): unchanged.
- **Assumptions (confirm at acceptance):** pane visible by default; preview refreshes when the model changes (throttled so large diagrams stay responsive); it follows the FR8 scale control because it renders the same model.
- **Format compatibility:** no — pure GUI addition.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace the FR6 row with the proposed wording (sequentially after CR-002 if that lands first).
- arc42 §8 (gui-app sub-modules): one-line addition of the preview pane to the app-glue list.
- No new ADR planned — reuses the existing resvg rasterization path (ADR 016); egui panel/splitter are standard widgets.
- Skeleton story **S-023** on `doc/stories/BOARD.md`, Why cites CR-004; no hard dependencies (exporters already done).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** 

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
