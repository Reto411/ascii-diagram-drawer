---
title: B-001 - ellipse-resizing-does-not-work
description: dragging the resize handle of a built-in ellipse changes nothing visible — the render pipeline stamps Pattern shapes at natural size, ignoring the instance extent
scope: gui-app / render-pipeline
status: fixed
severity: major
updated: 2026-10-31
---

# B-001: Ellipse resizing does not work

## Summary

On the main canvas (native build), selecting a built-in ellipse and dragging its resize handle produces no visible change: the oval stays 9×5 on the canvas, in the text output, and in SVG/PNG exports. The model's `CellSize` can silently change underneath (the gesture reaches `Diagram::resize`), but the render pipeline stamps `Pattern`-geometry shapes at their natural template size, so the rendered ellipse never reflects a resize. After such a "resize", the selection overlay's anchor dots also drift away from where connectors actually bind.

## Environment

| | |
|---|---|
| OS | unknown (native desktop build) |
| Target | native |
| Commit | `9331f63` |
| Other | — |

## Steps to reproduce

1. Open the app (native build) with any diagram, or open [repro.ad](assets/B-001/repro.ad) (one ellipse + one box + a connector between them).
2. Place an ellipse on the canvas (or select the existing one).
3. Click the ellipse body to select it — the small resize-handle square appears at the bottom-right cell of its extent.
4. Press that handle and drag outward (e.g. right/down by several cells), then release.

## Expected

The ellipse resizes grid-aligned, live during the drag, like a box does — per [S-013 AC1](../stories/S-013-gui-editing-gestures.md) ("Dragging a resize handle on a selected **built-in shape** changes its `CellSize`") and FR1 (place, move, resize).

## Actual

Nothing visible changes. The oval remains 9×5 in the canvas rendering, the text output, and SVG/PNG exports, no matter how far the handle is dragged. Headless repro shows the model size *does* change (`9×5 → 13×8`) while the rendered grid stays byte-identical — a silent model/render divergence that also corrupts what a saved `.ad` file means (it records an extent the output never shows).

## Evidence

> Public repo: no local paths, usernames, or credentials in any pasted output. Screenshots/logs/repro files live in `assets/B-001/`.

- [repro.ad](assets/B-001/repro.ad) — the reporter's file as saved (ellipse 9×5 + box 9×5 + connector `obj-1.e → obj-2.w`; note the ellipse is recorded at natural size).
- [screenshot.png](assets/B-001/screenshot.png) — canvas screenshot from the reporter during the repro.
- Headless repro (temporary test, deleted after evidence collection): build the reporter's diagram, apply exactly what the GUI gesture does (`Diagram::resize` to 13×8), render before/after:

  ```text
  $ cargo test -p render-pipeline --test tmp_bug_repro -- --nocapture
  model size after resize: 13x8
  rendered text byte-identical before/after resize: true
  --- BEFORE ---
     ___
    ,'   '.
   (       |          +-------+
    `.   .'|          |       |
      `-'  +--------->+       |
                       |       |
                       +-------+
  --- AFTER (model says 13x8) ---
     ___
    ,'   '.
   (       |          +-------+
    `.   .'|          |       |
      `-'  +--------->+       |
                       |       |
                       +-------+
  ```

- Anchor-dot drift after the same resize: the GUI would draw the ellipse's `e` anchor dot at cell (40,16) (proportional mapping onto the new 13×8 extent), while the pipeline resolves the connector endpoint at (36,14) (natural-size offset) — a 4×2-cell disagreement between the selection overlay and the rendered connectors.

## Investigation

> Filled by the AI while triaging (bug-report skill): dedupe result, docs check, repro attempts with commands + results. Speculation is labeled as speculation.

- **Duplicate:** none (bug board was empty at filing time).
- **Docs check:** **contradicts** [S-013 AC1](../stories/S-013-gui-editing-gestures.md) ("built-in shape" includes the ellipse) and FR1; **matches** [ADR 007 v3](../adr/007-user-defined-diagram-objects.md) point 4 + [C9 connector-routing](../arc42/12-concepts/connector-routing.md) (`Pattern` instances render at natural size: "template coordinates offset by the instance position") and [shape-template C7](../arc42/12-concepts/shape-template.md) ("All v1 shapes carry `resize_rules = None`"). The recorded design is internally inconsistent here: `gui-app/src/canvas.rs::hit_target` offers a resize handle for every unqualified type id (both built-ins), but the pipeline can only express resizes for `Rect` geometry (`place_rect` uses the instance pos/size; `place_pattern` stamps the fixed 9×5 art regardless). The `anchor_cell` doc in `canvas.rs` even assumes "fixed-size (`Pattern`) shapes have instance extent == template extent" — an invariant the GUI's own resize gesture breaks for the built-in ellipse.
- **Repro attempt:** `cargo test -p render-pipeline --test tmp_bug_repro -- --nocapture` → 2 passed; output quoted above (render byte-identical, model 13×8; dot/endpoint drift asserted) — reproduced: **yes**.
- *Speculation:* the reporter's saved file records the ellipse at natural size, so either they saved before resizing or the press landed on the oval body (move gesture) instead of the handle cell — for an ellipse the handle sits on a *blank* corner cell of the pattern, one cell right and one down from the last visible glyph, which makes it easy to miss. Either way, even a successful handle drag cannot produce a visible resize, so both paths land on the same user-visible failure.

## Resolution

> Filled when the bug closes: what fixed it (story S-NNN / commit hash), or why it won't be fixed. Empty while open.

**Fixed 2026-10-31.** Root cause confirmed as diagnosed: `ShapeType` had only two geometry kinds (`Rect`, `Pattern`) and the pipeline's `place.rs` matched on them — the built-in ellipse was a `Pattern`, so it was stamped opaquely at its natural 9×5 template size regardless of the instance's `CellSize`; resizing changed only the model. Fix (bounded context: render-pipeline + shape-registry, no GUI change needed):

1. `shape-registry`: new `GeometryKind::Ellipse` — the built-in ellipse now declares it (`builtin.rs`, `types.rs`).
2. `render-pipeline/src/ellipse.rs` (new): deterministic re-rasterizer — a pure function of `(w, h)` that reproduces the natural 9×5 template byte-for-byte at 9×5 and derives every other size structurally in the same glyph style (proportional caps, corner diagonal row pair, straight `(`/`)` sides).
3. `place.rs`: new `Ellipse` arm — stamps the re-rasterized outline at the instance pos/size, proportional anchor base edges, label over space cells.
4. `connectors.rs`: `Ellipse` endpoints resolve proportionally like `Rect` (shared `map_proportional`), so connectors stay bound to side midpoints at any size ([ADR 013 v3](../adr/013-connector-routing.md)).

Verified: the report's exact repro (ellipse resized 9×5 → 13×8) now renders a 13×8 oval — pinned by `resolve.rs::resized_ellipse_renders_at_instance_extent` plus place/connector regression tests and an exhaustive rasterizer property test. Anchor dots drift is resolved as a consequence: the rendered extent, the model extent, and the proportional anchor mapping all agree again. 
