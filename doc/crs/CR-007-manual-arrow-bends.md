---
title: CR-007 - user-modifiable arrow shapes (manual bends)
description: pressing onto an arrow selects it and dragging adds/moves intermediate bends (Visio-style), so users can manually route connectors for readable diagrams
scope: connectors (model + routing + GUI gestures)
status: accepted
priority: medium
updated: 2026-10-31
---

# CR-007: User-modifiable arrow shapes (manual bends)

## Requested change

Arrows (connectors) shall be **modifiable by pressing onto the arrow** to influence their shape, as is possible in tools such as Visio. Pressing an arrow selects it; dragging on it adds or moves intermediate bends (waypoints), so the user can manually route the arrow between its anchor points.

## Motivation

User need: "The user needs the possibility to shape the arrows to make nicely readable diagrams!" Automatic routing (straight line or single-corner L) cannot always produce a readable layout — e.g. routes crossing other shapes or labels; manual bend control is what diagram tools (Visio & co.) offer for this.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR9 | "Connectors (lines/arrows) can be drawn between/around shapes; they bind to shape anchors and stay attached when the bound shape moves or resizes" | "Connectors (lines/arrows) can be drawn between/around shapes; they bind to shape anchors and stay attached when the bound shape moves or resizes; the user can modify a connector's shape — pressing onto it selects it, and dragging adds/moves intermediate bends (waypoints) so arrows can be routed manually for readability; connectors without manual bends keep the automatic routing" |

## Conflict check

- **ADR 013 v2 (accepted):** "Connectors route orthogonally, with at most one corner … No multi-corner (Z/U) routes … The route is therefore a pure function of the two endpoints and their sides — no search." Its rejected alternative *multi-corner Manhattan routing* was shelved because it would need an automatic collision-avoidance policy "without a v1 use case". This CR introduces exactly that use case and dissolves the policy question: **the user defines the corners** — no automatic collision search. On acceptance: **ADR 013 → v3** (user-declared waypoints become model data; automatic straight/L routing remains for connectors without manual bends).
- **Concept C9 (connector routing):** updated to cover declared bends.
- **NFR3 (determinism):** holds — the route becomes a pure function of the two resolved endpoints *plus* the declared bends; still no search, no iteration order.
- **S-007 (done):** rework — hit-testing gains connector-body/bend targets, the gesture machine gains bend manipulation, routing consumes waypoints.

## Impact analysis

- **Stories:** S-007 (done — connectors) rework across `diagram-model` (connector definition gains optional bends), `ad-format` (`.ad` stores them), `render-pipeline` (routing with declared bends), `gui-app` (selection + drag gestures).
- **Assumptions (confirm at acceptance):** bends stay **orthogonal** (axis-aligned waypoints — the svgbob alphabet constraint of ADR 013 is untouched: no diagonals, no curves); connectors without manual bends keep today's straight/L auto-routing, so old files and existing diagrams render unchanged; declared bends persist in the model/file and follow anchor re-resolution when bound shapes move (FR9 preserved).
- **Format compatibility:** **yes** — `.ad` extension (optional per-connector bend data); ADR 004 round-trip / NFR2 at stake. Files without bends stay valid; unknown-statement tolerance already specified for the format.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace the FR9 row with the proposed wording.
- **ADR 013 → v3:** decision updated (user-declared waypoints; auto-routing for un-bent connectors); version-history entry noting the previously rejected alternative is now adopted *with user-defined corners*.
- Concept C9 (connector routing) updated; one line in the diagram-file-format concept for the bend statements.
- Skeleton story **S-026** on `doc/stories/BOARD.md`, Why cites CR-007; no hard dependencies (connectors exist since S-007).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-028** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board. ADR 013 → v4 (manual waypoint polylines; axis-aligned segments keep clean joins, explicit diagonals are `/` `\` content).

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
