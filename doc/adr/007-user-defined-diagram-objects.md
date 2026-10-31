---
title: 007 - user-defined-diagram-objects
description: users define their own diagram objects as named shape templates (plain-text ASCII art with anchor points) stored in shareable collections
scope: domain model / file format
status: accepted
updated: 2026-10-31
version: 6
---

# ADR 007: User-defined diagram objects and collections

## Context

[FR3](../arc42/14-important-requirements.md) (High): users must be able to define their own diagram objects **entirely themselves** — custom geometry/glyphs beyond the predefined set — and store them in named **collections** for grouping and reuse across diagrams ([§1.1 Obj. 3](../arc42/01-introduction-and-context.md#11-objectives)). Usage-model reference: asciilogic (canvas editing; connectors snap to shape anchors and stay bound, FR9).

Constraints in play: rendering must remain a **pure deterministic function** of the model (NFR3) on the svgbob-compatible grid ([ADR 001](001-glyph-resolution-alphabet.md)); storage is the open plain-text format family ([ADR 004](004-diagram-file-format.md)); custom objects must not break clean connector joins (glyph resolution, concept C1).

Open questions: what is a user-defined object *made of*, and how do custom objects interact with glyph resolution and connector binding?

## Decision

A **shape template** is the unit of user-defined content; a **collection** groups templates.

1. **Representation — stamped ASCII art.** A shape template is a named, fixed-size grid of svgbob-alphabet characters (the same alphabet as the resolved grid, ADR 001) plus declared **anchor points**: named cells where connectors may bind. Templates are authored by the user in plain text — the same look-and-feel as typing an ASCII diagram by hand; no separate DSL in v1.
   - **Anchors are per type** (v2): the anchor set is part of the template definition and shared by all instances of that shape. It can be defined in two equivalent modes from v1: **textually** in the plain-text template file, or **graphically** in a template editor — same underlying data, the file remains the source of truth (FR11).
2. **Collections.** Templates live in **collections**: named, open plain-text files (ADR 004 format family) that group related templates. Diagrams reference templates as `collection/name`. Collections make custom objects shareable between users and projects as ordinary files.
3. **Rendering — opaque stamp with live anchors.** At render time the template grid is stamped onto the cell grid at the object's position: non-anchor cells are copied verbatim (like text — they do *not* participate in glyph resolution), while anchor cells resolve against connecting edges like built-in shape borders, so connectors join cleanly and stay bound when the object moves (FR9).
4. **Resizability via declared rules.** A custom object is resizable only if its definition declares **resize behavior** (what to adjust when resized); without declared rules it is fixed-size — including unknown imported objects before registration (FR12). The rule mechanism (CR-003, concept C10 [resize-rules](../arc42/12-concepts/resize-rules.md)): a template declares **per-edge insert/remove characters** for growing/shrinking (top/bottom rows, left/right columns — e.g. a box inserts/removes `─`/`│` runs and keeps its corners), plus optional **parity variants** (alternate patterns selected by even/odd width or height) so curved shapes such as ellipses stay correct at every size. Re-rasterization is a pure function of (template, rules, target size) — NFR3 holds; the pipeline re-stamps the rasterized art at render time exactly like built-in `Ellipse` (B-001).
5. **Determinism.** Stamping + anchor resolution are pure functions of (template, position, connectors) — NFR3 holds; golden-file tests cover stamping and anchor joins headlessly (NFR5).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Define custom objects as geometry (rects/paths) like built-in shapes | Re-implements a mini vector/layout engine for user content; far more complex than the requirement needs; breaks the "type it like ASCII art" authoring model |
| Let template cells participate fully in glyph resolution (treat user art as live strokes) | User art vs renderer-owned junctions becomes ambiguous — unpredictable joins, hard to test; opaque stamping keeps custom content exactly as authored |
| Per-diagram inline definitions (no collections) | No grouping or reuse across diagrams — FR3 explicitly asks for collections; duplicates art in every file |

## Consequences

- **Positive:** trivially authorable (plain text, no DSL), shareable (plain files), deterministic; connectors still bind cleanly via anchors; no new rendering machinery beyond stamping + anchor resolution.
- **Negative / costs:** templates with dense art cannot merge borders with neighbours (by design — opaque stamps); collection naming and collision rules need a spec; resize rules express only per-edge row/column patterns — shapes whose interior changes non-uniformly when resized still need hand-authored variants.
- **Follow-ups:** ~~spec template + collection file syntax and anchor marker character(s)~~ done in S-003 — syntax is normative in [shape-template](../arc42/12-concepts/shape-template.md) (C7) and [collection](../arc42/12-concepts/collection.md) (C8); the anchor-marker question was settled by *coordinate-based* `anchor` declarations, so no marker character exists to collide with the svgbob alphabet. ~~implement collection loading~~ done in S-005 (parser/resolution) and S-016 (GUI discovery). ~~graphical anchor-definition mode in the template editor (FR11)~~ done in S-017 — `gui-app template_editor.rs` + `core-services registry_state.rs`, both authoring modes (textual file, graphical editor) write the same canonical `.adc` bytes. ~~spec the resize-rule declaration (format + re-rasterization algorithm)~~ settled by CR-003 (accepted 2026-10-31) — mechanism in concept C10 [resize-rules](../arc42/12-concepts/resize-rules.md), implementation S-024. Still open: preserve unknown statements of newer minor versions across GUI collection saves (v1 files are 1.0 and reject them, so nothing is lost today).

## Version history

- **v6 (2026-10-31):** CR-003 accepted — point 4 updated: the resize-rule mechanism is declared (per-edge insert/remove characters + optional even/odd parity variants, re-rasterized purely at render time like built-in `Ellipse`), concept C10 [resize-rules](../arc42/12-concepts/resize-rules.md) created; custom objects with rules become fully resizable (FR3 extended). Implementation: S-024.
- **v5 (2026-10-29):** status `proposed` → `accepted`. All v1-scope pieces are implemented: templates + collections as data (S-005), stamping with live anchors in the pipeline (S-006/S-007), GUI collection discovery + placement (S-016), and the template editor with graphical anchor definition (S-017 — `gui-app template_editor.rs` over the seam's new `ShapeCatalog::register`/`Renderer::preview_pattern`, ADR 009 v6). Decision unchanged.
- **v4 (2026-10-23):** open spec points settled by S-003 — template/collection file syntax and the `collection/name` resolution rule are now normative in concept docs C7/C8; anchors are declared by coordinates (`anchor <name> at <x> <y>`), eliminating the marker-character collision concern. Decision unchanged; status stays `proposed` until S-005 implements it.
- **v3 (2026-10-18):** point 4 reworked — resizing is important: a custom object is resizable once its definition declares resize behavior; unknown imported shapes stay fixed-size until defined/registered (resolves the v2 conflict with FR12). Rule support scoped to v1+.
- **v2 (2026-10-18):** anchor sets are per type and definable both textually and graphically from v1 (product interview, recorded as FR11); flagged the resize-rule conflict with FR12's registration flow (open, v1+).
- **v1 (2026-10-18):** initial decision — shape templates as stamped ASCII art with declared anchors, grouped in collections.
