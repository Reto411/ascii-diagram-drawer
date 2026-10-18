---
title: 007 - user-defined-diagram-objects
description: users define their own diagram objects as named shape templates (plain-text ASCII art with anchor points) stored in shareable collections
scope: domain model / file format
status: proposed
updated: 2026-10-18
supersedes: null
---

# ADR 007: User-defined diagram objects and collections

## Context

[FR3](../arc42/14-important-requirements.md) (High): users must be able to define their own diagram objects **entirely themselves** — custom geometry/glyphs beyond the predefined set — and store them in named **collections** for grouping and reuse across diagrams ([§1.1 Obj. 3](../arc42/01-introduction-and-context.md#11-objectives)). Usage-model reference: asciilogic (canvas editing; connectors snap to shape anchors and stay bound, FR9).

Constraints in play: rendering must remain a **pure deterministic function** of the model (NFR3) on the svgbob-compatible grid ([ADR 006](006-svgbob-compatible-ascii-alphabet.md)); storage is the open plain-text format family ([ADR 004](004-diagram-file-format.md)); custom objects must not break clean connector joins (glyph resolution, concept C1).

Open questions: what is a user-defined object *made of*, and how do custom objects interact with glyph resolution and connector binding?

## Decision

A **shape template** is the unit of user-defined content; a **collection** groups templates.

1. **Representation — stamped ASCII art.** A shape template is a named, fixed-size grid of svgbob-alphabet characters (the same alphabet as the resolved grid, ADR 006) plus declared **anchor points**: named cells where connectors may bind. Templates are authored by the user in plain text — the same look-and-feel as typing an ASCII diagram by hand; no separate DSL in v1.
2. **Collections.** Templates live in **collections**: named, open plain-text files (ADR 004 format family) that group related templates. Diagrams reference templates as `collection/name`. Collections make custom objects shareable between users and projects as ordinary files.
3. **Rendering — opaque stamp with live anchors.** At render time the template grid is stamped onto the cell grid at the object's position: non-anchor cells are copied verbatim (like text — they do *not* participate in glyph resolution), while anchor cells resolve against connecting edges like built-in shape borders, so connectors join cleanly and stay bound when the object moves (FR9).
4. **Fixed size in v1.** Custom objects are not resizable; only predefined shapes (FR1) resize. Resizing a stamp would require re-rasterizing user art — out of scope for v1.
5. **Determinism.** Stamping + anchor resolution are pure functions of (template, position, connectors) — NFR3 holds; golden-file tests cover stamping and anchor joins headlessly (NFR5).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Define custom objects as geometry (rects/paths) like built-in shapes | Re-implements a mini vector/layout engine for user content; far more complex than the requirement needs; breaks the "type it like ASCII art" authoring model |
| Let template cells participate fully in glyph resolution (treat user art as live strokes) | User art vs renderer-owned junctions becomes ambiguous — unpredictable joins, hard to test; opaque stamping keeps custom content exactly as authored |
| Per-diagram inline definitions (no collections) | No grouping or reuse across diagrams — FR3 explicitly asks for collections; duplicates art in every file |

## Consequences

- **Positive:** trivially authorable (plain text, no DSL), shareable (plain files), deterministic; connectors still bind cleanly via anchors; no new rendering machinery beyond stamping + anchor resolution.
- **Negative / costs:** custom objects are fixed-size in v1 (visible limitation vs built-in shapes); templates with dense art cannot merge borders with neighbours (by design — opaque stamps); collection naming and collision rules need a spec.
- **Follow-ups:** spec template + collection file syntax and the anchor marker character(s) (must not collide with svgbob drawing characters or the double-quote escape) in ADR 004's format chapter; on acceptance: concept docs for "shape template" and "collection" (§12), GUI editing flow for templates, `collection/name` resolution rules.
