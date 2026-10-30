---
title: CR-012 - shape filter in the template sidebar
description: a text filter narrows the shape list in the template sidebar by name — keeps selection usable as the number of native shapes grows
scope: gui (template sidebar)
status: proposed
priority: low
updated: 2026-10-30
---

# CR-012: Shape filter in the template sidebar

## Requested change

There shall be a **filter functionality for the shapes** — a text input in the template sidebar that narrows the visible shape list while typing.

## Motivation

User: "since there will be many native shapes later on" — as the predefined shape set grows, scanning an ever-longer sidebar list becomes impractical; a name filter makes any shape findable in one keystroke regardless of list length.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR6 | "UI consists of a template sidebar + editor canvas (minimal)" | append to the sidebar part: "…a template sidebar **with a filter input that narrows the shape list by name**, and an editor canvas (minimal)" |
| NFR4 | "Low learning curve / minimal UI complexity: …" | unchanged — contextually supported (findability) |

Note: CR-002 and CR-011 (both pending) also reword FR6. If all are accepted, the **combined final FR6 text is applied once**, at the last of the acceptances.

## Conflict check

- **ADR 008 (egui/eframe):** a single-line `egui::TextEdit` plus client-side filtering over an in-memory list — standard widgets on all four targets, no contradiction.
- **CR-011 interaction:** complementary — the filter works on top of the collapsible categories/subcategories (groups with no matching shape are hidden while a filter is active). If CR-011 is rejected, the filter simply applies to the flat list.
- **Shape registry / model / format / pipeline:** untouched — filtering is client-side over the registered set; nothing is stored or serialized.

## Impact analysis

- **Stories:** `gui-app` only — sidebar gains a filter input above the shape list; the list renders the filtered subset (live, per keystroke).
- **Assumptions (confirm at acceptance):** case-insensitive substring match on the shape name; filter is transient (not persisted); empty groups hidden while filtering; no other attributes (category, tags) are matched in v1.
- **Format compatibility:** no — pure GUI.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: extend the FR6 row with the proposed wording (merged into the combined text if CR-002/CR-011 land first).
- No new ADR planned (standard egui widget, no architectural choice).
- Skeleton story **S-031** on `doc/stories/BOARD.md`, Why cites CR-012; no hard dependencies — pairs with CR-011's categories when that lands.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** 

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
