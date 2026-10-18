---
title: glyph-resolution
description: how the renderer picks one svgbob-alphabet character per grid cell when multiple edges meet — read before working on text rendering
scope: rendering
status: draft
updated: 2026-10-18
---

# Concept: Glyph Resolution

## Definition

The process by which the renderer assigns exactly **one** svgbob input-alphabet character ([ADR 001](../../adr/001-glyph-resolution-alphabet.md)) to each occupied grid cell, based on all edges and content passing through that cell.

## Why it exists

Characters occupy single cells; where shape borders meet or cross (corners, T-junctions, crossings), several logical edges share one cell. Without resolution, joins would render as broken or overlapping glyphs. Decided in [ADR 001](../../adr/001-glyph-resolution-alphabet.md): v1 fixed the mechanism (one character per cell, resolved from all edges/content passing through it), v2 fixed the target alphabet (svgbob's input set).

## How it works / behaves

- Input: per-cell set of connections (N/E/S/W edge presence) + any non-edge content
- Output: the matching glyph, e.g. straight line → `-`/`|`, sharp corner/junction (2–4 connections) → `+`, rounded corners → `. , ' `` ` `, dashed lines → `- - -` / `~` / `_` / `:` / `!`, arrow ends → `< > ^ v V`, ports/circles → `* o O`
- The renderer always searches for a matching character **wherever possible** (ADR 001 v1); the target set is svgbob's documented input alphabet (ADR 001 v2)
- Fallback rules for ambiguous cells (no single glyph fits, e.g. text under an edge): **[TBD — open]**

## Relations

- **Cell grid** — the coordinate system glyph resolution operates on (concept doc not yet written; will live in this folder)
- **Diagram model** — shapes declare edges; they never choose glyphs themselves

## Implementation

Not yet implemented. Expected home: platform-independent rendering crate (see AGENTS.md conventions).
