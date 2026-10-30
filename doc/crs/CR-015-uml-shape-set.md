---
title: CR-015 - uml-shape-set
description: built-in catalog ships the core UML shape set (class/use-case/sequence/component/deployment/state/activity) — the requester's daily standard as a system architect
scope: shapes/templates
status: proposed
priority: high
updated: 2026-10-30
---

# CR-015: UML shape set in the built-in catalog

## Requested change

The predefined catalog ships the **core UML shape set** in addition to the current box and ellipse — shapes for the standard UML diagram types (class, use case, sequence, component, deployment, state machine, activity). Every new shape is a built-in with standard anchors (FR11), rendered as verbatim glyph art within the svgbob alphabet. Shapes carry geometry only; UML semantics (compartments, stereotypes, labels) come from shape-owned text boxes (CR-016/FR16) and free text (FR10).

## Motivation

The requester is a system architect who draws a lot of UML diagrams — UML is the common standard for their work. With only box and ellipse available, every UML-specific shape (actor, lollipop, component, node, diamond, …) would have to be hand-crafted as a custom template per project; shipping the standard set makes the tool immediately useful for its main use case.

## Requirement impact

> Exact current wording from arc42 §14 (or "new — proposed FR-n" for additions). One row per affected requirement; "unchanged" when only contextually affected.

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| **FR15** | — new (allocated after pending CR-013/CR-014's proposed FR13/FR14; IDs are appended, never renumbered) — | "The predefined catalog covers the core UML shape set for the standard diagram types (class, use case, sequence, component, deployment, state machine, activity): at minimum interface lollipop, actor, activation bar, component, port, deployment node, artifact, rounded-rectangle state, decision diamond, initial and final state nodes — in addition to the existing box and ellipse. Every new shape is a built-in with standard anchors (FR11); shapes carry geometry only, UML semantics come from shape-owned text boxes (FR16) and free text (FR10). The exact glyph art and per-shape anchor sets are finalized in the implementing story." |
| FR1 | "User can place, move, and resize predefined shapes (boxes, circles, …) on a canvas" | unchanged — contextually affected: the predefined set grows per FR15 |
| FR11 | "…predefined shapes ship with standard default anchors (basic positions + official UML connection points); the exact default anchor set per predefined shape is an open spec point" | unchanged — contextually affected: that open spec point gets concretized by FR15's implementing story |

## Conflict check

> Does this contradict a recorded decision? Cite ADRs/concepts/arc42 chapters. If yes: state exactly which decision changes and how (ADR version bump + history entry). "None" when clean.

- **ADR 007 (user-defined objects)**: none — built-ins are code, not files; new shapes join the `shape-registry` built-in catalog as data (S-005 "built-ins as data"). Consistent, no bump.
- **ADR 001 v2 / C1 glyph resolution**: none — the svgbob alphabet includes dashes (`~ _ : ! - - -`), rounded corners (`. , ' `` ` `), ports/circles (`* o O`) and arrows; all UML art is expressible as verbatim pattern art (the same mechanism the built-in ellipse's oval uses).
- **FR11 open spec point** ("exact default anchor set per predefined shape"): partially resolved by the implementing story — noted, no wording change.

## Impact analysis

> Filled by the AI while clarifying the CR (change-request skill): affected stories with IDs, file-format compatibility risk. Decision-relevant only — no implementation scoping (crates/effort are decided at handoff by the cr-implementation skill).

- **Stories:** no done story needs rework — purely additive catalog data; S-016's sidebar lists new built-ins automatically (Built-in group). **Dependency (requester-stated): requires CR-003** (template creation via text box + resize-rule toolset) — it has a huge impact on shape definition; S-024 starts only after CR-003 is accepted and implemented. Compartmented UML shapes (class, component) additionally require CR-016 (shape-owned text boxes).
- **Format compatibility:** no format change (type ids are already arbitrary strings; new built-in ids are additive). Forward-compatibility caveat only: files using the new shapes need a newer app version to resolve them — same class of behavior as any catalog growth, low risk.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

On acceptance, one commit:

1. §14: add FR15 (appended after pending CR-013/CR-014's proposed FR13/FR14; FR1/FR11 unchanged).
2. ADRs: none.
3. Concepts: `shape-template.md` (C7) Implementation note updated — expanded built-in set; `overview.md` tracker row.
4. Story board: skeleton **S-024** "shape-registry — UML built-in shape set (FR15)" — Why cites CR-015; acceptance criteria written by the cr-implementation skill at start. Dependencies: **CR-003 (accepted + implemented, per requester)** and **CR-016** for the compartmented shapes.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-015 as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** Requester (2026-10-30): "This will require CR-003 since it has a huge impact on shape definition!" — recorded as a hard dependency above.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
