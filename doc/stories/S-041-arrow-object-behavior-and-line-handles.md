---
title: S-041 - arrow-object-behavior-and-line-handles
description: CR-020 — connectors behave as normal objects (body drag moves the whole arrow), visible draggable endpoint/bend handles, double-click bend insertion, in-canvas tip dropdown per end
scope: gui-app
status: backlog
updated: 2026-11-01
---

# S-041: Arrow object behavior and Visio-style line handles

## Why

[CR-020](../crs/CR-020-arrow-object-behavior-and-line-handles.md) (accepted 2026-11-01, FR9): arrows are the least intuitive objects on the canvas — they cannot be moved like shapes, their endpoints give no visible affordance, adding bends is a hidden gesture, and tip selection is an undocumented press-cycle. They shall behave as normal objects with Visio-style line handles: drag body moves the whole arrow; selected arrows show visible square handles at both endpoints and every bend, all draggable; double-click on a segment inserts a bend; pressing a selected endpoint opens an in-canvas tip dropdown. Both connector kinds (shape-to-shape and standalone).

## Dependencies

- S-028 (done) — manual bends: body-drag gesture reworked to whole-arrow move, double-click insertion added
- S-029 (done) — per-end tips: press-to-cycle replaced by the in-canvas dropdown trigger
- S-031 (done) — free endpoints: endpoint drag/detach/reattach semantics extended with visible handles

## Acceptance criteria

> Written by the cr-implementation skill when it starts, from CR-020's requested change (items 1–5). Feature bar: at least one `ui_decomposition_*` test drawing a representative diagram through the real UI — including placing arrows and reshaping them via the new handles.

## Implementation notes

> Filled in while working the story: approach, files touched, decisions made (link ADRs), deviations from plan.

- 

## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [ ] AC1 — 
- [ ] AC2 —
