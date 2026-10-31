---
title: CR-003 - template creation via text box + resize-rule toolset
description: custom template authoring = hand-typed ASCII art in a text box, plus a second toolset declaring resize behavior (per-edge insert/remove characters, parity variants for ellipses)
scope: shapes/templates (custom object authoring + resize rules)
status: accepted
priority: high
updated: 2026-10-31
---

# CR-003: Template creation via text box + resize-rule toolset

## Requested change

Creating a custom template shall consist of **(1)** a text box (on top) in which the user adds the ASCII art of the object by hand, and **(2)** a second box/toolset for defining the resizing behaviour — declaring which character is inserted when the object grows and which character is removed when it shrinks (per edge/dimension). The toolset shall include an option to define **parity rules** (alternate patterns for even/odd sizes) so that curved shapes such as ellipses — where every second size renders slightly differently — can be defined correctly.

## Motivation

User feedback: painting the object cell-by-cell in the graphical grid editor is clunky compared to simply typing/pasting the art; and custom objects are currently **fixed-size** (ADR 007 v3), so a user cannot define their own resizable shape at all — FR3's "define your own diagram objects entirely" is only half-realized. This also resolves ADR 007's open follow-up ("spec the resize-rule declaration — format + re-rasterization algorithm — when scoping FR12") ahead of the import work, with a concrete mechanism instead of an open spec point.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR3 | "Users can define their own diagram objects entirely (custom geometry/glyphs beyond the predefined set) and store them in named collections for grouping and reuse across diagrams" | "Users can define their own diagram objects entirely (custom geometry/glyphs beyond the predefined set) and store them in named collections for grouping and reuse across diagrams; custom objects can declare resize behavior — per-edge character insertion/removal rules for growing/shrinking, plus optional parity variants (alternate patterns for even/odd sizes) so curved shapes stay correct at every size" |
| FR12 | "…registration declares anchor points and resize behavior, which makes the shape fully editable (**v1+**, not in v1 scope)" | unchanged — contextually affected: its "resize behavior" declaration now has a specified mechanism (this CR) |

## Conflict check

- **ADR 007 v3, point 4 + open follow-up:** "A custom object is resizable only if its definition declares resize behavior … rule support lands in **v1+**; in v1 all custom objects are fixed-size" and follow-up "spec the resize-rule declaration (format + re-rasterization algorithm) when scoping FR12". This CR specifies that mechanism now, ahead of FR12. On acceptance: **ADR 007 version bump (v4)** — point 4 updated with the declared-rule mechanism; the follow-up is marked done by CR-003's story.
- **Concept C7 (shape-template):** "Resize rules: none in v1 … A future `resize` statement inside a template block is the reserved extension point" — this CR materializes that reserved extension; C7 updated and a new concept doc for the resize-rule mechanism created (**C10**).
- **S-017 (done):** the graphical cell-grid editor is the current authoring surface; the text box becomes primary. Anchor declaration (FR11: textual + graphical) is kept — flagged as an assumption, confirm at acceptance.
- **NFR3 (determinism):** no conflict — resizing a rule-declared custom shape must remain a pure function of model + declared rules (implementation constraint).

## Impact analysis

- **Stories:** S-005 (done — collection parsing; `shape-registry`/`.adc` format) — the `.adc` file gains optional resize-rule statements incl. parity variants, with parser/serializer round-trip. Render-pipeline (S-006 done) — re-rasterization algorithm for rule-declared custom shapes (today: opaque fixed-size stamp). S-013 (done — gestures) — the resize handle must be offered for custom templates *with* declared rules (currently built-ins only). S-017 (done — template editor) — creation dialog reworked: art text box on top + resize toolset below.
- **Format compatibility:** **yes** — `.adc` extension (new optional statements); ADR 004 round-trip / NFR2 at stake. Files without rules stay valid and fixed-size; unknown-statement tolerance already specified in C7 for newer versions.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace the FR3 row with the proposed wording.
- **ADR 007 → v4:** point 4 updated (rule mechanism declared: per-edge insert/remove characters + parity variants); open follow-up closed; version-history entry.
- New concept doc **C10 `resize-rules.md`** from `doc/templates/concept.md` + §12 index row; C7 shape-template "Resize rules" section updated to point at C10.
- Skeleton story **S-022** on `doc/stories/BOARD.md`, Why cites CR-003; no hard dependencies — spans `.adc` format (shape-registry), re-rasterization (render-pipeline), dialog + gesture enablement (gui-app).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-024** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board. ADR 007 → v6; new concept C10 [resize-rules](../arc42/12-concepts/resize-rules.md); `resize` statements join the `.adc` minor-version 1.1 batch.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
