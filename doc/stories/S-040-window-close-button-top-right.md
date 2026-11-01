---
title: S-040 - window-close-button-top-right
description: CR-019 — app-bar window controls ordered minimize, fullscreen, close (close at the top-right corner)
scope: gui-app
status: backlog
updated: 2026-11-01
---

# S-040: Window close button at the top right (rightmost of the app-bar controls)

## Why

[CR-019](../crs/CR-019-close-button-at-top-right.md) (accepted 2026-11-01, FR14 / [ADR 017 v2](../adr/017-material-design-title-bar.md)): on KDE Plasma — as on Windows — the platform convention places close at the right end of the window controls. Today the custom Material app bar orders the trio **close ×, fullscreen □, minimize ─**, so minimize occupies the top-right corner and the left-end close invites mis-clicks. The buttons shall read left-to-right: minimize, fullscreen, close.

## Dependencies

- S-027 (done) — the app bar this story reorders (`gui-app/src/appbar.rs`, `window_controls`)

## Acceptance criteria

> Written by the cr-implementation skill when it starts, from CR-019's requested change: left-to-right order minimize → fullscreen → close pinned in a real frame (tier-0b), commands/glyphs unchanged (S-027 AC3 assertions stay valid), Material goldens regenerated. Feature bar: at least one `ui_decomposition_*` test drawing a representative diagram through the real UI.

## Implementation notes

> Filled in while working the story: approach, files touched, decisions made (link ADRs), deviations from plan.

- 

## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [ ] AC1 — 
- [ ] AC2 —
