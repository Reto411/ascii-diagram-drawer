---
title: B-002 - scale-duplicates-shape
description: pressing the toolbar "+" scale control shows each shape twice — once at its original size and once scaled — persisting across presses (×2, ×4)
scope: gui-app / render-pipeline
status: as-designed
severity: major
updated: 2026-10-31
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

**Fix analysis 2026-10-31:**

- **The attached screenshots do not show duplication.** Pixel-level decode of all three shots (luminance scan + glyph reconstruction against the grid pitch ≈9.8px × 19px): each canvas contains exactly **one** object — an ellipse. `scale-after-x2.png` is the exact FR8 ×2 expansion of the 9×5 built-in ellipse template (`   ___   ` / ` ,'   '. ` / `(       )` / ` `.   .' ` / `   `-'   `): every glyph became a 2×2 block, one copy, no unscaled remnant. `scale-after-x4.png` is the exact FR8 **×3** expansion of the same template.
- **The described canvas content is absent from the evidence.** The report says the canvas held an ellipse + box + solid connector (the B-001 repro file), but no box or connector ink exists anywhere on the canvas in any of the three shots — only the ellipse.
- **The app state is internally consistent at ×3, not ×4.** In `scale-after-x4.png` both the toolbar label and the bottom status bar read “Scale ×3”, and the rendered expansion is exactly ×3. The status bar shows the last executed action (`Scale ×<n>` from the `+`/`−` handlers — the only two code paths that mutate scale), so the app registered exactly two increments, not three. Whether one press was lost in the reporter's environment or miscounted cannot be determined from static screenshots.
- **No duplication path exists in code.** The canvas has a single text render pass (`canvas_frame()` → `render_pipeline::render` → `expand(grid, scale)` → `to_text`, one galley per row); selection/placement overlays are strokes/fills, never shape copies. Scale expansion happens once, at the end of the pipeline (FR8), and is unit-tested in `render-pipeline` (`scale_two_*`, `scale_three_*`).
- **Fresh headless repro at HEAD:** rendering the B-001 repro file's object set (ellipse + box + connector) through `render_pipeline::render` at ×1/×2/×4 produces exactly one scaled copy at every scale — pipeline output correct.
- **New tier-0b regression test** (`gui_tests::b002_scale_presses_render_exactly_one_expanded_copy`): places an ellipse through the real UI, presses `+` three times — all three register (toolbar reaches “Scale ×4”, ruling out lost clicks in the app's input path) — and after each press asserts the drawn canvas rows equal the exact FR8 expansion of the ×1 baseline. Passes at HEAD; a mutation drawing every row twice fails it immediately, so it guards precisely against the reported symptom.
- **Verdict:** observed behavior matches FR8 exactly (cell → N×N block, single scaled rendering) and S-015's scale control (model-level `diagram.scale()`, canvas + exports follow). The filed expectation — “only the scaled copy should be visible” — is what the software actually does. Closing **as-designed**; if a different scaling behavior is wanted, that is a change request, not a defect.

## Resolution

**Closed as-designed (2026-10-31).** No defect found: the reported duplication does not appear in any of the attached screenshots — each shows exactly one rendering of a single ellipse, and the ×2/×4 shots are pixel-exact FR8 expansions (the last shot is ×3, matching its “Scale ×3” toolbar label *and* status bar, i.e. the app state is fully consistent). The described box + connector are absent from all three shots. Pipeline output at HEAD is correct at every scale; the new tier-0b test `gui_tests::b002_scale_presses_render_exactly_one_expanded_copy` drives the real UI (place ellipse → three `+` presses) and pins that the canvas always shows exactly one scaled copy — it fails immediately if rows are drawn twice. Behavior matches [FR8](../arc42/14-important-requirements.md) (scale control scales the rendered output; cell → N×N block) as implemented by S-015's model-level scale control; no code change made. If a different scaling behavior is desired, file a change request.
