---
title: CR-013 - prerendered-shape-previews
description: shape selection shows a pre-rendered visual preview of each shape (same bytes as stamped on the canvas) instead of name + tiny text snippet — WYSIWYG picking
scope: gui
status: accepted
priority: medium
updated: 2026-10-31
---

# CR-013: Pre-rendered shape previews in the template sidebar

## Requested change

Each shape in the template sidebar (shape selection) is shown as a **pre-rendered visual preview** of its glyph pattern — an actual rendering of the shape, not just its name next to a small monospace text snippet. The preview is generated from the same pattern bytes the renderer stamps on placement, so what the user sees when picking a shape is exactly what appears on the canvas ("the user gets what he sees"). Previews stay legible for templates of any size within the fixed sidebar width.

## Motivation

The current picker lists each shape as a name label plus its raw glyph rows drawn at 9 pt monospace, native cell size (S-016 AC1). That snippet is hard to recognize — especially for larger custom templates, which overflow the 180 px sidebar — and it is not visually identical to how the same shape renders on the canvas (the canvas draws the exact `to_text` bytes at its own measured glyph advance, S-012). A user picking a shape cannot reliably tell what it will look like once placed. Pre-rendered previews remove that guesswork and directly serve NFR4/Q4 (low learning curve: minimal UI that shows you what you get).

## Requirement impact

> Exact current wording from arc42 §14 (or "new — proposed FR-n" for additions). One row per affected requirement; "unchanged" when only contextually affected.

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR6 | "UI consists of a template sidebar + editor canvas (minimal)" | unchanged — contextually affected: the sidebar's shape presentation is refined by new FR13; the "minimal" constraint is untouched |
| **FR13** | — new (next free ID after FR12) — | "Shapes in the template sidebar are shown as pre-rendered previews of their glyph pattern, generated from the same pattern bytes the renderer stamps on placement — what the user sees when picking a shape is exactly what gets placed (WYSIWYG). Previews stay legible for templates of any size within the sidebar width." |
| NFR4 | "Low learning curve / minimal UI complexity: a new user can compose a first diagram without documentation" | unchanged — contextually affected (improved by FR13) |

## Conflict check

> Does this contradict a recorded decision? Cite ADRs/concepts/arc42 chapters. If yes: state exactly which decision changes and how (ADR version bump + history entry). "None" when clean.

- **ADR 009 v5** (`ShapeInfo.preview: Vec<String>`): the seam carries the glyph pattern rows *verbatim* "so the sidebar can show a small preview without touching registry types". No conflict if pre-rendering happens **GUI-side from those same bytes** — the seam stays unchanged and the WYSIWYG property (preview bytes == stamped bytes, already pinned in S-016/S-017) is preserved. If an implementation instead needs rasterized images or layout data to cross the seam, ADR 009 gets a version bump (→ v8) + history entry — flag at handoff, not assumed here.
- **NFR3 (determinism)**: no conflict — previews derive from the same deterministic pattern bytes; the requirement reinforces "same bytes as stamped".
- **ADR 008 (egui/eframe, native + wasm)**: no conflict — previews must render on both targets without platform forks (egui text layout is available on both); noted as a constraint for S-019.
- **Concept docs**: no concept doc defines the sidebar's preview presentation; `collection.md` (Implementation) mentions "the grouped sidebar with glyph previews" — wording updated on acceptance.

## Impact analysis

> Filled by the AI while clarifying the CR (change-request skill): affected stories with IDs, file-format compatibility risk. Decision-relevant only — no implementation scoping (crates/effort are decided at handoff by the cr-implementation skill).

- **Stories:** S-016 (done) — rework: its AC1 pinned the current 9 pt monospace text presentation in `app.rs`; only the drawing changes, grouping/placement behavior and the pure view model (`sidebar.rs`) stay. S-017 (done) — the template editor's live preview pane is a separate window; contextually related, no change required. S-019 (backlog, wasm milestone) — the preview mechanism must work on wasm32 (constraint, no scope change).
- **Format compatibility:** no — pure GUI presentation change; ADR 004 round-trip / NFR2 not at stake.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

On acceptance, one commit:

1. §14: add FR13 (new ID appended after FR12; FR6/NFR4 unchanged).
2. ADRs: none while the seam stays as-is (GUI-side rendering of `ShapeInfo.preview`); ADR 009 → v8 only if rasterized data must cross the seam.
3. Concepts: `collection.md` Implementation note "grouped sidebar with glyph previews" → pre-rendered previews; `overview.md` tracker row for the new story.
4. Story board: skeleton **S-022** "gui-app — pre-rendered shape previews in the template sidebar (FR13)" — Why cites CR-013, scope = sidebar drawing only; acceptance criteria written by the cr-implementation skill at start. Dependencies: none beyond done stories (S-016).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-013 as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-034** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
