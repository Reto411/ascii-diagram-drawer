---
title: CR-014 - canvas-panning
description: user can pan the canvas to navigate diagrams that extend beyond the visible area (at 1x large diagrams are clipped and off-screen parts unreachable)
scope: gui
status: accepted
priority: high
updated: 2026-10-31
---

# CR-014: Canvas panning for diagram navigation

## Requested change

The user can **pan the canvas view** to navigate a diagram that extends beyond the visible area. At scale 1× a large diagram is not fully visible, and there is currently no way to reach or see the off-screen parts. Panning shifts which part of the model space is displayed; it never changes the diagram model or its rendering (pure navigation). All existing canvas gestures (place, select, move, connect, resize) keep working at any pan offset.

## Motivation

At scale 1× a diagram larger than the window shows only its top-left part: the rest is clipped and **unreachable** — the user cannot see it, and off-screen shapes cannot even be clicked to select or move them (the canvas is a fixed central panel; the viewport origin is pinned to a margin, S-012). Panning is the minimal navigation capability that makes large diagrams usable. It complements FR8's scale control: scaling changes the rendered output (spacing), panning changes what is visible.

## Requirement impact

> Exact current wording from arc42 §14 (or "new — proposed FR-n" for additions). One row per affected requirement; "unchanged" when only contextually affected.

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| **FR14** | — new (allocated after CR-013's proposed FR13 to keep pending IDs distinct; IDs are appended, never renumbered) — | "The user can pan the canvas to navigate a diagram that extends beyond the visible area. Panning only changes which part of the model space is displayed; it never changes the diagram model or its rendering, and all canvas gestures keep working at any pan offset." |
| FR8 | "A scale control scales the entire diagram's rendered output up (larger spacing → more readable text output), not just a display zoom" | unchanged — contextually affected: panning complements scaling (pan = what is visible; scale = rendered output) |
| NFR3 | "No automatic layout rearrangement — element positions are exactly where the user placed them; rendering is a **pure deterministic function** of the diagram model (no randomness, no external state); same model → byte-identical output" | unchanged — contextually affected: the pan offset is view state and must stay out of the render input |

## Conflict check

> Does this contradict a recorded decision? Cite ADRs/concepts/arc42 chapters. If yes: state exactly which decision changes and how (ADR version bump + history entry). "None" when clean.

- **ADR 009 (service seam)**: none — panning is GUI viewport state (`gui-app`'s `Viewport.origin_x/y`); nothing crosses the seam.
- **S-013 gesture machine** (drag on a shape = select+move, drop-on-anchor = connect): a pan interaction must be disambiguated from these existing drag gestures — a constraint for the implementation handoff; no recorded ADR or concept covers gesture bindings, so nothing is contradicted.
- **NFR3 (determinism)**: no conflict as long as the pan offset stays view-only (stated in FR14).
- **ADR 008 (egui, native + wasm)**: none — panning works on both targets with standard egui input events; no platform forks.
- **Concept docs**: none define the canvas viewport (implementation detail of `gui-app`); nothing to update.

## Impact analysis

> Filled by the AI while clarifying the CR (change-request skill): affected stories with IDs, file-format compatibility risk. Decision-relevant only — no implementation scoping (crates/effort are decided at handoff by the cr-implementation skill).

- **Stories:** S-012 (done) — rework: the viewport origin becomes mutable view state; its mapping/hit-test helpers already take a `Viewport`, so this is mostly wiring plus input handling. S-013 (done) — regression risk: the new pan interaction must not steal shape drag/connect gestures. No other stories affected.
- **Format compatibility:** no — no model, file-format, or rendering impact; ADR 004 round-trip / NFR2 not at stake.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

On acceptance, one commit:

1. §14: add FR14 (appended after CR-013's proposed FR13; FR8/NFR3 unchanged).
2. ADRs: none.
3. Concepts: none — the viewport is not a documented concept.
4. Story board: skeleton **S-023** "gui-app — canvas panning for diagram navigation (FR14)" — Why cites CR-014; acceptance criteria written by the cr-implementation skill at start. Dependencies: none beyond done stories (S-012/S-013).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-014 as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-022** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
