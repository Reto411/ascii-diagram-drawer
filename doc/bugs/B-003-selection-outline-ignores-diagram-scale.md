---
title: B-003 - selection-outline-ignores-diagram-scale
description: at diagram scale > 1 the selection/move outline of a selected object keeps its ×1 geometry while the rendered content expands into N×N blocks — the outline no longer encloses the object, and clicks on the scaled part outside the unscaled extent do not hit it
scope: gui-app
status: reproduced
severity: major
updated: 2026-10-30
---

# B-003: Selection outline ignores diagram scale

## Summary

On the main canvas, once the whole-diagram scale (FR8 toolbar `+` control) is above ×1, the blue selection/move outline around a selected object **stops following the rendered content**: it keeps exactly its ×1 pixel geometry while the object's glyphs expand into N×N blocks. At Scale ×2 an 11-cell text spans 22 rendered cells but the outline still covers only the original 11 — it floats over the top-left part of the rendered text instead of enclosing it; at ×4 the disagreement is 4×. Reproduced headlessly with **both** a Text and a Box shape. Related symptom in the same area: hit-testing is unscaled as well — at ×2, clicking on visible glyphs in the right half of a scaled text object does **not** select it.

## Environment

| | |
|---|---|
| OS | unknown (native desktop build) |
| Target | native |
| Commit | headless repro on `989d51d`; reporter's session assumed same line as B-001/B-002 (`9331f63`) |
| Other | Reporter's canvas: a Text shape among other objects, Scale > 1 (exact value not stated; screenshot shows the outline misaligned) |

## Steps to reproduce

1. Open the app (native build), or open [repro.ad](assets/B-003/repro.ad) (one text object `hello world`).
2. Place a Text shape and type content (e.g. `hello world`), or select the existing one.
3. Click the text to select it — at Scale ×1 the blue outline exactly encloses the rendered glyphs.
4. Click the toolbar `+` scale control (FR8): Scale ×1 → ×2, repeat for ×4.

## Expected

Per FR8 ([§14](../arc42/14-important-requirements.md)), scaling expands every grid cell into an N×N block — the rendered object grows proportionally, and only the scaled rendering is shown (see also B-002). The selection overlay must keep enclosing the selected object's **rendered** extent at any scale (at ×1 the GUI guarantees "the outline covers exactly the selected cells"), and per [S-018 AC2](../stories/S-018-gui-free-text.md) text objects stay selectable/draggable with the same rules as shapes — which includes being hit by clicks on their rendered body.

## Actual

The outline's pixel rect is **identical before and after scaling** while the content grows: at ×2 it covers only the top-left 1× of an N×-sized rendering (text: 11×1 cells outlined vs 22×2 rendered; box: 9×5 vs 18×10), so it no longer encloses the object. At ×4 the same fixed rect vs a 44×4 text extent. Additionally, at ×2 a click on visible glyphs in the right half of the scaled text (canvas cells beyond the unscaled model extent) does not select the object — only clicks inside the original ×1 footprint do.

## Evidence

> Public repo: no local paths, usernames, or credentials in any pasted output. Screenshots/logs/repro files live in `assets/B-003/`.

- [screenshot.png](assets/B-003/screenshot.png) — reporter's canvas at Scale > 1 with a selected Text shape. Vision-model reading (local Qwen3.8 + mmproj): the selection outline is drawn about one grid row *above* the glyphs, offset right from the first glyph, and only ~3–4 cells wide while the text spans 8+ — i.e. it floats over the beginning of the text instead of enclosing it; no other anomalies on the canvas.
- [repro.ad](assets/B-003/repro.ad) — minimal file for manual repro (open, select the text, press `+`).
- Headless repro (temporary tests in the `gui-app` `Gui` frame-pumping harness, deleted after evidence collection): place Text `hello world` at cell (5,3), select it, click toolbar `+`; measured the SELECTION-colored stroke rect each frame:

  ```text
  $ cargo test -p gui-app tmp_bug_repro -- --nocapture
  [x1] selection [[238.1 89.0] - [330.9 105.0]] | expected [[238.1 89.0] - [330.9 105.0]] | text galley Some([[238.1 89.0] - [330.9 105.0]])
  [x2] selection [[238.1 89.0] - [330.9 105.0]] | expected [[238.1 89.0] - [423.6 121.0]] | scale label: "Scale ×2"
  [x2] canvas galley "h e l l o   w o r l d" at (238,89)-(415,105)
  [x4] selection [[238.1 89.0] - [330.9 105.0]] | expected [[238.1 89.0] - [609.0 153.0]]
  ```

  The rendered text itself scales correctly (one scaled copy, galley spans 2× the width at ×2 — consistent with B-002 *not* reproducing headlessly); only the selection rect stays frozen at its ×1 geometry. Same for a Box:

  ```text
  [box x1] selection [[238.1 89.0] - [314.0 169.0]] | expected [[238.1 89.0] - [314.0 169.0]]
  [box x2] selection [[238.1 89.0] - [314.0 169.0]] | expected [[238.1 89.0] - [389.8 249.0]]
  ```

- Related hit-testing symptom (same harness, text at ×2 — rendered glyphs span canvas cols 5..26, unscaled model extent is cols 5..15):

  ```text
  [click x2] left half (col 7): selected=true
  [click x2] right half (col 20): selected=false
  ```

## Investigation

> Filled by the AI while clarifying (bug-report skill): dedupe result, docs check (as-designed detection only), repro attempts with commands + results showing observed vs expected. Clarification only — no root cause analysis, no fix design (those belong to the bug-fix skill at handoff; its findings are appended here as a dated "Fix analysis" block).

- **Duplicate:** none — B-001 is the ellipse *resize handle* doing nothing (render pipeline stamps Pattern shapes at natural size); B-002 is duplicated scaled copies of shapes (not reproduced headlessly, and this repro's canvas shows exactly one scaled copy, so it does not interfere here). This bug is the selection overlay + hit-testing ignoring the scale factor.
- **Docs check:** contradicts FR8 (scale changes the rendered extent — overlays tracking that object must track it too) and S-018 AC2 / S-013 selection-overlay intent (outline around the selected object's extent; text selectable on its body). No ADR or concept doc describes a scale-independent selection outline → not as-designed.
- **Repro attempt:** headless, `cargo test -p gui-app tmp_bug_repro -- --nocapture` — reproduced: **yes** (text at ×2 and ×4, box at ×2, plus the click-miss above). The reporter's screenshot matches the headless geometry.

## Resolution

> Filled when the bug closes: what fixed it (story S-NNN / commit hash), or why it won't be fixed. Empty while open.

-
