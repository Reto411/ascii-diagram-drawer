---
title: S-029-connector-selectable-tips
description: CR-008 — selectable per-end tips + arrow anchor markers (FR9) (skeleton; ACs at implementation)
scope: render-pipeline/gui-app
status: backlog
updated: 2026-10-31
---

# S-029: render-pipeline + gui-app — selectable per-end tips + arrow anchor markers (FR9)

## Why

FR9 extended by CR-008: each connector end carries a selectable tip style — none / hollow arrow / empty rhombus `◇` / filled rhombus `◆` — changed by pressing onto the tip; files without tip data keep the legacy default (source none, target arrow) so existing renders stay byte-identical. GUI anchor dots become small arrow symbols signalling 'connecting adds an arrow'. ADR 013 v5; rhombi are content-only glyphs (no ADR 001 bump).

## Dependencies

- S-028 — same module; tip hit-testing sits on the route

## Acceptance criteria

> Skeleton — acceptance criteria are written by the cr-implementation skill when work starts, from [CR-008](CR-008-selectable-arrow-tips.md)'s "Requested change" + §14 wording. Nothing here is agreed yet beyond the requirement itself.

- (TBD at implementation)

## Implementation notes

- 

## Verification

- 
