---
title: CR-018 - resize-handle-symbol
description: the shape's resize hook is drawn with a dedicated resize symbol (distinct from anchor dots) so users can see it as the drag-to-resize affordance
scope: gui/interaction
status: accepted
priority: low
updated: 2026-10-31
---

# CR-018: Resize symbol on the shape's resize hook

## Requested change

The resize hook of a selected shape (its bottom-right handle cell, S-014/S-015) is drawn with a **dedicated resize symbol** — visually distinct from the anchor dots — so that the user can see at a glance that this mark is for increasing/changing the size of the shape by dragging. The exact glyph and color (e.g. diagonal arrows or a corner marker in a non-anchor color) are finalized in the implementing story; the requirement is that it clearly reads as "drag to resize" and is not confused with an anchor dot.

## Motivation

Usability: today the handle is rendered as a 6×6 px filled square in the *same color* as the anchor dots (`draw_overlays`, gui-app). It does not communicate its purpose — users cannot discover that dragging it resizes the shape, so the resize feature (FR2) effectively stays hidden. A recognizable resize symbol makes the affordance visible.

## Requirement impact

> Exact current wording from arc42 §14 (or "new — proposed FR-n" for additions). One row per affected requirement; "unchanged" when only contextually affected.

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| **FR18** | — new (allocated after pending CR-013…CR-017's proposed FR13–FR17; IDs are appended, never renumbered) — | "The resize handle of the selected shape is drawn with a dedicated resize symbol that is visually distinct from anchor dots and clearly conveys drag-to-resize. The exact glyph and color are specified in the implementing story." |
| FR2 | "User can move and resize placed shapes on the canvas" | unchanged — contextually affected: the resize affordance becomes discoverable per FR18 |

## Conflict check

> Does this contradict a recorded decision? Cite ADRs/concepts/arc42 chapters. If yes: state exactly which decision changes and how (ADR version bump + history entry). "None" when clean.

- **S-014/S-015 resize handle (v1, bottom-right cell)**: no conflict — the *hit target* stays the same cell (`canvas.rs` `hit_target`/`overlays` unchanged); only the visual marker in `draw_overlays` changes. The existing invariant "the marker sits exactly on the hit cell" (marker/hit parity, see the "handle drawn where it does not work" bug note) must be preserved.
- **ADR 009 seam / NFR3 determinism**: none — pure GUI rendering of view state; nothing crosses the service seam, no model or format change.

## Impact analysis

> Filled by the AI while clarifying the CR (change-request skill): affected stories with IDs, file-format compatibility risk. Decision-relevant only — no implementation scoping (crates/effort are decided at handoff by the cr-implementation skill).

- **Stories:** S-014/S-015 (resize handle + arrowheads, done) — small rework of `draw_overlays` in gui-app (marker pixels only); hit logic in `canvas.rs` untouched. Golden PNG screenshots (gui-testing skill) covering the selection overlay will need regeneration.
- **Format compatibility:** none — no `.ad`/`.adc` change, nothing crosses the seam.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

On acceptance, one commit:

1. §14: add FR18 (appended after pending CR-013…CR-017's proposed FR13–FR17); FR2 unchanged.
2. ADRs: none.
3. Concepts: none — rendering detail of an existing interaction, not a concept.
4. Story board: skeleton **S-027** "gui-app — resize handle symbol (FR18)" — Why cites CR-018; acceptance criteria written by the cr-implementation skill at start.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-018 as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-038** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
