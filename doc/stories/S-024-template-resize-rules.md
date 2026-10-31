---
title: S-024-template-resize-rules
description: CR-003 — resize rules for custom templates (FR3) (skeleton; ACs at implementation)
scope: shape-registry/render-pipeline/gui-app
status: backlog
updated: 2026-10-31
---

# S-024: shape-registry + render-pipeline + gui-app — resize rules for custom templates (FR3)

## Why

FR3 extended by CR-003: custom objects are currently fixed-size (ADR 007 v3), so 'define your own diagram objects entirely' is only half-realized. Templates gain per-edge insert/remove resize rules + optional even/odd parity variants; re-rasterization is a pure function at render time. Mechanism normative in C10 [resize-rules](../arc42/12-concepts/resize-rules.md), decided in ADR 007 v6; `resize` statements join the `.adc` minor-version 1.1 batch (C6). Template authoring also gains the hand-typed text box + rule toolset from CR-003's request.

## Dependencies

- none beyond done stories (S-005/S-006/S-017)

## Acceptance criteria

> Skeleton — acceptance criteria are written by the cr-implementation skill when work starts, from [CR-003](CR-003-template-textbox-resize-rules.md)'s "Requested change" + §14 wording. Nothing here is agreed yet beyond the requirement itself.

- (TBD at implementation)

## Implementation notes

- 

## Verification

- 
