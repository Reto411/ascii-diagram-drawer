---
title: CR-002 - grouped dropdown menus in the toolbar
description: replace the flat toolbar with a menu bar of dropdown menus grouping actions by category (e.g. File) — less clutter as features grow
scope: gui (toolbar / menu structure)
status: proposed
priority: low
updated: 2026-10-30
---

# CR-002: Grouped dropdown menus in the toolbar

## Requested change

The top toolbar shall be a **menu bar with dropdown menus that group actions by category** — e.g. a **File** menu containing all file-related options (Open…, Save, Save As…, Copy, text output, export). One-shot actions live inside the dropdowns instead of as flat buttons; the menu structure is extensible so future feature areas get their own menus without adding more top-level buttons.

## Motivation

User feedback: the toolbar is already a long flat row and will keep growing as features land (collections, custom templates, import — FR3/FR12). Grouped dropdowns keep the visible UI minimal and predictable, matching common desktop conventions (NFR4: low learning curve / minimal UI complexity).

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR6 | "UI consists of a template sidebar + editor canvas (minimal)" | "UI consists of a top menu bar with dropdown menus that group actions by category (e.g. File), a template sidebar, and an editor canvas (minimal)" |
| NFR4 | "Low learning curve / minimal UI complexity: a new user can compose a first diagram without documentation" | unchanged — contextually supported: the visible surface stays flat while actions grow |

## Conflict check

- **ADR 008** (egui/eframe): menu bars / dropdowns are standard egui widgets (`egui::menu`, top panel) on all four targets — consistent, no contradiction.
- **S-012…S-018 GUI stories** built the current flat toolbar in `gui-app/src/app.rs` (toolbar row: Open…/Save/Save As…/Copy/Text .txt/Export SVG…/Export PNG… + Scale control). Not an ADR — the toolbar layout is reworked by the implementing story; interaction logic (S-013 gestures, S-018 text editing) is untouched.
- **Interaction with CR-001** (if accepted): orthogonal and order-independent — the File menu simply hosts whatever actions exist at implementation time; with CR-001 that is Open…/Save/Save As…/Copy/Export… (single export dialog).

## Impact analysis

- **Stories:** S-012 (done — scaffold + toolbar) rework: flat button row becomes a menu bar. S-014/S-015 (done): their buttons move into the File menu, behavior unchanged. S-018 (done): text tool lives in the sidebar — unaffected. S-019 (backlog — wasm milestone): egui menus work identically on wasm; no regression.
- **Scope note:** the Scale control (`Scale ×n −/+`) is a persistent live control, not a one-shot action — it stays outside the dropdowns (top-right), flagged for confirmation at acceptance.
- **Format compatibility:** no — pure GUI structure change.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace the FR6 row with the proposed wording.
- arc42 §8 (gui-app sub-modules): record the menu bar as part of the app glue / `io` UX layer — one-line update.
- No new ADR planned (standard egui widget, no architectural choice).
- Skeleton story **S-021** on `doc/stories/BOARD.md`, Why cites CR-002; no new dependencies; must not regress S-019's web path.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** 

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
