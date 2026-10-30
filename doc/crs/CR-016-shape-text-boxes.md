---
title: CR-016 - shape-text-boxes
description: shape templates can declare named text regions (text boxes) with position/extent that belong to the shape — definable in file and editor; generalizes the single label
scope: shapes/templates/format
status: proposed
priority: high
updated: 2026-10-30
---

# CR-016: Shape-owned text boxes (definable text regions on templates)

## Requested change

Shape templates can declare one or more **text boxes** — named text regions with a position and extent within the shape's pattern. Each region **belongs to the shape**: instances carry per-region content, and regions move and resize together with the shape. Regions are defined as part of the template — both textually in the template file and graphically in the template editor (mirroring anchors, FR11). This generalizes the current single implicit shape label into N definable regions; e.g. a UML class with name/attributes/methods compartments.

## Motivation

UML shapes need multiple bound text areas (class compartments, actor names, component labels). Free-text objects (FR10) do not belong to the shape: they don't move or resize with it and must be placed manually per instance. The requester — a system architect drawing UML daily — needs shape-owned text boxes that can be *defined* as part of the shape; this is also a prerequisite for CR-015's compartmented UML shapes (class, component).

## Requirement impact

> Exact current wording from arc42 §14 (or "new — proposed FR-n" for additions). One row per affected requirement; "unchanged" when only contextually affected.

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| **FR16** | — new (allocated after pending CR-013/CR-014/CR-015's proposed FR13/FR14/FR15; IDs are appended, never renumbered) — | "Shape templates can declare one or more named text regions (text boxes) with a position and extent within the shape. Instances carry per-region content, and regions move and resize together with the shape. Regions are definable both textually in the template file and graphically in the template editor (mirroring anchors, FR11). The existing single shape label remains supported." |
| FR3 | "Users can fully define their own diagram objects (custom geometry/glyphs beyond the predefined set) and store them in named collections for grouping and reuse across diagrams" | unchanged — contextually affected: template definitions gain text regions (FR16) |
| FR4 | "Diagrams are saved to and loaded from the open plain-text file format (round-trip: reload → identical model)" | contextually affected — **high risk**: per-region content must be encoded in `.ad` and round-trip byte-identically; template region declarations extend the `.adc` grammar |
| FR10 | "Free text can be placed as an element on the canvas (independent of shapes)" | unchanged — complementary: free text stays independent; FR16's boxes are shape-bound |

## Conflict check

> Does this contradict a recorded decision? Cite ADRs/concepts/arc42 chapters. If yes: state exactly which decision changes and how (ADR version bump + history entry). "None" when clean.

- **ADR 004 v3 (accepted) / C6 `diagram-file-format` / C7+C8 template & collection grammars**: this is a **format change** — `.adc` gains a text-region declaration statement (analogous to `anchor at x y`), and `.ad` gains per-region content on shape instances. On acceptance: ADR 004 → v4 with history entry, C6/C7/C8 grammar extensions, and `ad-format` parser/serializer changes with round-trip proof (proptest). The existing versioning mechanism (minor bump = skip-and-preserve unknown statements) keeps old readers tolerant of new files; the existing single `label` statement must keep parsing identically (backward compatibility).
- **FR11 precedent**: anchors are already definable "both textually in the template file and graphically in the template editor" — FR16 mirrors that established pattern exactly. No conflict.
- **NFR3 determinism**: none — regions are static template data plus instance content; rendering stays a pure function of the model.

## Impact analysis

> Filled by the AI while clarifying the CR (change-request skill): affected stories with IDs, file-format compatibility risk. Decision-relevant only — no implementation scoping (crates/effort are decided at handoff by the cr-implementation skill).

- **Stories:** S-004 (ad-format, done) — parser/serializer extension; round-trip regression risk (proptest suite must keep passing). S-006 (render-pipeline, done) — region rendering generalizes the label clipping/wrapping rules (C1 F3/F4). S-017 (template editor, done) — rework: the editor gains region definition alongside anchor mode. **Blocks** CR-015's compartmented UML shapes.
- **Format compatibility:** **yes — high risk.** ADR 004 round-trip / NFR2 directly affected; versioning + skip-and-preserve must be extended and proven, not assumed.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

On acceptance, one commit:

1. §14: add FR16 (appended after pending CR-013/CR-014/CR-015's proposed FR13/FR14/FR15); FR3/FR4/FR10 unchanged.
2. ADRs: **ADR 004 → v4** with Version history entry (format extension: template text-region declarations + per-region instance content; versioning/skip-and-preserve extended). No new ADR needed.
3. Concepts: `diagram-file-format.md` (C6 — instance grammar), `shape-template.md` (C7 — region declaration + Implementation), `collection.md` (C8 — carries templates); `overview.md` tracker row.
4. Story board: skeleton **S-025** "Shape-owned text boxes — format, model, template editor (FR16)" — Why cites CR-016; acceptance criteria written by the cr-implementation skill at start. No dependencies beyond done stories; blocks S-024's compartmented shapes.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-016 as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** 

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
