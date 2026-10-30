---
title: CR-017 - multi-select-group-move
description: select several shapes with Ctrl/Shift+click and move them as a group — standard editor behavior for working with related elements
scope: gui/interaction
status: proposed
priority: medium
updated: 2026-10-30
---

# CR-017: Multi-select (Ctrl/Shift+click) and group move

## Requested change

The user can select **several shapes** in the GUI with **Ctrl or Shift + mouse click**, and **move a group of shapes around**: dragging any selected member moves the entire selection by the same delta. Delete removes every selected object together with its connectors; Escape clears the whole selection.

## Motivation

Today only one object can be selected at a time (`EditorState.selection` is a single id; S-013 click = select + drag). Repositioning a set of related shapes — e.g. shifting a whole cluster of classes or moving actors next to their use cases — requires selecting and dragging each shape individually, which is tedious for the requester's daily UML work. Multi-select with group move is standard editor behavior users expect.

## Requirement impact

> Exact current wording from arc42 §14 (or "new — proposed FR-n" for additions). One row per affected requirement; "unchanged" when only contextually affected.

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| **FR17** | — new (allocated after pending CR-013/CR-014/CR-015/CR-016's proposed FR13–FR16; IDs are appended, never renumbered) — | "Multiple objects can be selected simultaneously in the GUI by Ctrl- or Shift-clicking them, and the whole selection can be moved as a group: dragging any selected member moves every member by the same delta. Delete removes every selected object together with its connectors; Escape clears the selection." |
| FR2 | "User can move and resize placed shapes on the canvas" | unchanged — contextually affected: moving extends to groups per FR17 |

## Conflict check

> Does this contradict a recorded decision? Cite ADRs/concepts/arc42 chapters. If yes: state exactly which decision changes and how (ADR version bump + history entry). "None" when clean.

- **S-013 interaction contract** ("click on shape = select + start drag; drop on anchor dot = connect"): preserved as the base case — an unmodified click behaves exactly as today (single selection). Modifier-clicks extend, not change, it. No conflict.
- **ADR 009 seam / NFR3 determinism**: none — selection is pure GUI view state (`EditorState`), nothing crosses the service seam, and rendering stays a pure function of model + view state.
- **Resize (S-014/S-015 bottom-right handle)**: v1 keeps resize single-selection only; group resize is explicitly out of scope for this CR (can be a follow-up).

## Impact analysis

> Filled by the AI while clarifying the CR (change-request skill): affected stories with IDs, file-format compatibility risk. Decision-relevant only — no implementation scoping (crates/effort are decided at handoff by the cr-implementation skill).

- **Stories:** S-013 (drag/click interaction, done) — rework: `EditorState`'s single `selection: Option<Id>` becomes a selection set; pointer handling gains modifier-key awareness (egui modifiers); dragging applies one delta to every member. The selection overlay (currently drawn around the single selected object's bounds) and anchor-dot offering extend to all members. Delete already cascades per object (object + its connectors) — extended to the whole set.
- **Format compatibility:** none — pure view state, no `.ad`/`.adc` change, nothing crosses the seam.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

On acceptance, one commit:

1. §14: add FR17 (appended after pending CR-013…CR-016's proposed FR13–FR16); FR2 unchanged.
2. ADRs: none.
3. Concepts: none — selection is GUI view state, not a documented concept.
4. Story board: skeleton **S-026** "gui-app — multi-select (Ctrl/Shift+click) + group move (FR17)" — Why cites CR-017; acceptance criteria written by the cr-implementation skill at start.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-017 as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** 

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
