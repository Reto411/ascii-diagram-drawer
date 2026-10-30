---
title: B-002 - scale-duplicates-shape
description: pressing the toolbar "+" scale control shows each shape twice — once at its original size and once scaled — persisting across presses (×2, ×4)
scope: gui-app / render-pipeline
status: open
severity: major
updated: 2026-10-30
---

# B-002: Scale duplicates shape

## Summary

On the main canvas, clicking the toolbar `+` scale control (FR8) leaves each shape displayed **twice**: once as the original (unscaled) and once resized (scaled). The duplication persists across further presses (×2 → ×4) — it is a stable on-canvas state, not a one-frame flash. Which of the reporter's objects appears duplicated is visible in the attached screenshots; the defect itself is scale-triggered, so any shape should be affected.

## Environment

| | |
|---|---|
| OS | unknown (native desktop build) |
| Target | native (same session/build as B-001, `9331f63`; not explicitly reconfirmed for this report) |
| Commit | `9331f63` (assumed — same running app as the B-001 session) |
| Other | Reporter's canvas: one ellipse + one box + a solid connector between them (the B-001 repro file, `image`) |

## Steps to reproduce

1. Open the app with a diagram containing at least one shape (reporter's canvas: ellipse + box + connector).
2. Click the `+` scale control in the top bar (Scale ×1 → ×2).
3. Observe the main canvas. Repeat `+` for ×4.

## Expected

Per FR8 ([§14](../arc42/14-important-requirements.md)): scaling expands every grid cell into an N×N block of repeated characters — the existing diagram grows, wider spacing makes it more readable. Only the scaled copy should be visible; the unscaled rendering must not remain on the canvas.

## Actual

Each shape is displayed duplicated: **once as the original and once resized**, simultaneously. The state persists across presses (screenshots at ×2 and ×4 below).

## Evidence

> Public repo: no local paths, usernames, or credentials in any pasted output. Screenshots/logs/repro files live in `assets/B-NNN/`.

- Before (×1): [scale-before-x1.png](assets/B-002/scale-before-x1.png)
- After one `+` (×2): [scale-after-x2.png](assets/B-002/scale-after-x2.png)
- After three presses (×4): [scale-after-x4.png](assets/B-002/scale-after-x4.png)

## Investigation

> Filled by the AI while clarifying (bug-report skill): dedupe result, docs check (as-designed detection only), repro attempts with commands + results showing observed vs expected. Clarification only — no root cause analysis, no fix design (those belong to the bug-fix skill at handoff; its findings are appended here as a dated "Fix analysis" block).

- **Duplicate:** none (B-001 is ellipse *resizing* via drag handle; this is scale-triggered duplication)
- **Docs check:** contradicts FR8 — cell expansion into N×N blocks implies a single, scaled rendering; no ADR/concept describes keeping the unscaled copy visible → not as-designed
- **Repro attempt (headless):** NOT reproduced. Render-pipeline output for clean diagrams at ×2/×4 is correct (each character expanded to an N×N block, no extra copies). Headless egui frame pumping (`gui-app`, `Gui` harness) of: fresh box ×2, fresh ellipse ×2, selected ellipse ×2, B-001-state resized ellipse ×2, resized box ×2, fresh box ×4 — in every case the painted canvas text contained exactly one (scaled) copy and no unscaled row. The trigger therefore depends on session state not covered by those attempts; the reporter's screenshots are the primary evidence.

## Resolution

> Filled when the bug closes: what fixed it (story S-NNN / commit hash), or why it won't be fixed. Empty while open.

-
