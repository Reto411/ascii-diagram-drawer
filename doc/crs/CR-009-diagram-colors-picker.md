---
title: CR-009 - diagram colors via color picker (svgbob settings)
description: background, shape fill and line/text colors per diagram — chosen with a color picker remembering the last 16 used colors; rendered on canvas and in SVG/PNG via svgbob Settings
scope: rendering (color) + model/format + exporters + GUI
status: accepted
priority: medium
updated: 2026-10-31
---

# CR-009: Diagram colors with color picker

## Requested change

**Shapes shall allow changing their background color**, and **text elements shall allow changing their color as well**. Selecting a color opens a **color picker**; the picker **saves the last 16 used colors** for quick re-selection. The user directed that svgbob's own color capability be used ("svgbob allows to select the color, use that!") and confirmed the resulting granularity: **diagram-level colors** (Option A below) — "for the moment".

## Motivation

Colored diagrams are more readable (emphasis, grouping, semantics per region) and fit the modern look requested in CR-006; colors must survive into SVG/PNG export, not just the canvas.

## Scope (per user decision 2026-10-30: use svgbob's color capability)

**Diagram-level colors, three roles mapping 1:1 onto svgbob `Settings`** (verified in the pinned crate source, svgbob 0.7 — `to_svg_with_settings`):

| Role | svgbob `Settings` field | Effect |
|------|------------------------|--------|
| Background | `background` | backdrop rect + unfilled areas |
| Shape fill | `fill_color` | filled shape fragments (shape "backgrounds") |
| Line & text | `stroke_color` | strokes of all geometry **and** fill of all text |

- Chosen with a **color picker dialog** (one shared history of the **last 16 used colors**); applied on the **canvas** and to **SVG/PNG exports**; stored with the diagram in `.ad`.
- **Per-element colors** (each shape/text its own color) are explicitly **out of scope "for the moment"**: not expressible in svgbob's API (global-only, verified). If wanted later, that is a follow-up CR with its own export decision (canvas-only display or SVG post-processing).

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR15 | new — no current requirement covers color | "A diagram can have a background color, a shape fill color and a line/text color, chosen with a color picker that remembers the last 16 used colors; the colors are stored with the diagram and shown on the canvas and in SVG/PNG exports (via svgbob `Settings`)" |
| FR2 | "Diagram is exported as plain monospace text … copy/paste-able anywhere" | unchanged — plain text cannot carry color; `.txt`/clipboard stays monochrome |
| FR5 | "…vector output is derived from the resolved character grid via svgbob ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)), so exports always match the text representation exactly" | unchanged — geometry/content interpretation is untouched; colors are presentation attributes applied from stored model data (see conflict check) |

## Conflict check

- **FR5 invariant:** holds — the vector interpretation of the grid is the same geometry; color is presentation data from the model, not from the text.
- **ADR 004 / NFR2:** `.ad` gains optional color data → format extension; files without colors stay valid and render exactly as today (defaults = current monochrome look).
- **NFR3 (determinism):** holds — colors are model data; rendering stays a pure function of the model.
- **CR-005 interaction:** the picker's "last 16 used colors" is a user preference → persisted in the application config file once CR-005 lands; until then session-only (flag at acceptance).

## Impact analysis

- **Stories:** `diagram-model` (optional diagram-level color triple), `ad-format` (optional color block, round-trip), `exporters` (switch from default `to_svg` to `to_svg_with_settings` with `Settings` built from the model; PNG follows via the rasterizer), `gui-app` (canvas applies the colors; new color-picker dialog with 16 recent swatches; entry point in the toolbar/menu). The CR-004 preview pane shows colored SVG for free.
- **Assumptions (confirm at acceptance):** one shared picker and one shared 16-swatch history across the three roles; default = no stored colors (today's look); plain-text output always monochrome (FR2).
- **Format compatibility:** **yes** — `.ad` extension (optional color data); ADR 004 round-trip / NFR2 at stake; old files stay valid.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: append **FR15** with the proposed wording.
- No new ADR: this uses ADR 005's existing mechanism (`svgbob` as vector renderer) through its `Settings` parameter; the diagram-level granularity is recorded in this CR (user decision 2026-10-30, "for the moment").
- One line each: `diagram-file-format` concept (optional color block) and `render-pipeline` concept (colors applied via `Settings` at export).
- Skeleton story **S-028** on `doc/stories/BOARD.md`, Why cites CR-009; no hard dependencies (16-swatch persistence pairs with CR-005's config file when that lands).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-030** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
