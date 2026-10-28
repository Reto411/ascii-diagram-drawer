---
title: render-pipeline
description: how the diagram model becomes outputs — resolved character grid first, then text/SVG/PNG from that single grid (svgbob-style interpretation); read before working on rendering or export
scope: rendering
status: stable
updated: 2026-10-26
---

# Concept: Render pipeline

## Definition

The deterministic path from diagram model to output: objects with explicit grid coordinates → **resolved character grid** (one glyph per occupied cell) → outputs. Text is the grid itself; SVG/PNG are produced by interpreting that same grid (svgbob-style), so all outputs match exactly.

## Why it exists

- **Single source of truth for all outputs.** Deriving vector output from the resolved grid — not from raw object geometry — guarantees SVG/PNG always look like the text ([FR5](../14-important-requirements.md)).
- **Pure deterministic logic** ([Q1](../01-introduction-and-context.md#12-quality-goals-top-level) / NFR3): no layout engine, no randomness; same model → byte-identical text *and* geometrically identical vector output.
- One grid stage serves all outputs; the grid itself uses svgbob's input alphabet ([ADR 001](../../adr/001-glyph-resolution-alphabet.md)), so SVG is a direct, headlessly unit-testable (NFR5) call into the proven `svgbob` crate — no adapter in between ([ADR 005](../../adr/005-use-svgbob-as-vector-renderer.md)).

## How it works / behaves

Three stages, all pure functions:

1. **Place** — objects are stamped onto the cell grid at their stored coordinates (no layout pass); connectors re-resolve their anchors and stamp routed edges + arrowhead ([connector routing, C9](connector-routing.md)).
2. **Resolve** — every occupied cell gets exactly one glyph via [glyph resolution](glyph-resolution.md) (C1), based on the N/E/S/W connections and content passing through the cell.
3. **Emit** — per output family:
   - *Text*: dump the resolved grid (UTF-8, LF) — written rule: rows joined with `\n`, each row's trailing whitespace trimmed, no final newline; the empty grid exports as the empty string.
   - *SVG* (svgbob-style interpretation, [ADR 005](../../adr/005-use-svgbob-as-vector-renderer.md)): the resolved grid **is** svgbob input ([ADR 001](../../adr/001-glyph-resolution-alphabet.md)) — it is passed unchanged to `svgbob::to_svg`, and the interpretation (per-cell fragments, merging of collinear fragments, promotion to high-level shapes such as rects and circles) is delegated to the `svgbob` crate.
   - *PNG*: rasterization of the produced SVG (rasterizer TBD).

The character grid is a **derived intermediate representation** — it is never stored as the file format ([ADR 004](../../adr/004-diagram-file-format.md)).

## Relations

- **Glyph resolution (C1)** — stage 2 of this pipeline; the per-cell lookup that makes both text and vector output deterministic
- **Diagram file format** (ADR 004) — defines the *stored* model; this pipeline consumes it, producing the grid only at render time
- **Connector routing (C9)** — the connector branch of the place stage: anchor references re-resolve to cells on every render, so moved/resized shapes carry their connectors; routing decision in [ADR 013](../../adr/013-connector-routing.md) (`doc/arc42/12-concepts/connector-routing.md`)
- **WYSIWYG canvas** ([§8.4.7](../08-building-block-view.md)) — the GUI canvas renders this *same* resolved grid in a monospace font (the exact bytes of the text output) with selection/anchor overlays on top, so the view can never diverge from the export

## Implementation

Implemented in `crates/render-pipeline` (headless, no windowing or I/O), shared by all GUI targets and the CLI/export paths. The GUI will reach it only through the `Renderer` service trait of the seam ([ADR 009](../../adr/009-ui-api-service-layer.md)) — the pipeline itself stays a pure function; the seam wraps it.

- `render(&Diagram, &ShapeRegistry) -> Result<ResolvedGrid, RenderError>` — the public entry point: place → resolve → scale-expand (FR8).
- `place.rs` — stage 1: stamps objects in document order at their stored coordinates (no layout pass). `Rect` geometry (box) derives borders from the instance pos/size; `Pattern` geometry (ellipse, custom templates) stamps verbatim art with passive anchor base edges (ADR 007); free text stamps verbatim with quote escaping.
- `connectors.rs` — stage 1 (connector branch, C9): endpoint resolution from anchor references (proportional `Rect` mapping, offset `Pattern` mapping), straight/single-corner L routing, structural-edge stamping, arrowhead at the target end ([ADR 013](../../adr/013-connector-routing.md)).
- `resolve.rs` — stage 2: [glyph resolution](glyph-resolution.md) (C1), one svgbob-alphabet glyph per cell; trims to the occupied extent.
- `grid.rs` — `ResolvedGrid` (the derived intermediate, never stored as source data) and `expand(&ResolvedGrid, scale)` — line glyphs fill their whole scale block so edges stay continuous; content glyphs land in a defined sub-cell.
- Emission lives in `crates/exporters`: `to_text(&ResolvedGrid) -> String` (the rule above) and `to_svg(&ResolvedGrid) -> Result<String, ExportError>` — svgbob consumes the *exact* bytes of `to_text`, so text and SVG input are identical by construction; out-of-domain input degrades to literal text rather than failing (svgbob 0.7.6's public API is infallible — `ExportError::Svgbob` is the forward-compat seam, ADR 005). PNG emission follows in S-009.
- Export goldens: `crates/exporters/tests/golden/*.txt|*.svg` (box, box+label, junction, connector with arrowhead, empty) pin exact bytes; re-export byte-stability is unit-tested.
- Tests: per-rule-table unit tests, golden files under `tests/golden/*.grid` (incl. five connector scenarios), and proptest properties for determinism (AC1) and alphabet purity (AC4) over random diagrams with connectors.
