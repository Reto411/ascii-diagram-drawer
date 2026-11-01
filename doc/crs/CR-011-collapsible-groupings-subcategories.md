---
title: CR-011 - collapsible groupings with subcategories
description: UI groupings (menu bar dropdowns, template sidebar) become collapsible and support subcategory nesting — tidier selection as the number of actions and shape types grows
scope: gui (menu / sidebar structure)
status: implemented
priority: low
updated: 2026-10-31
---

# CR-011: Collapsible groupings with subcategories

## Requested change

**Groupings shall be collapsible** and **shall give the options to subcategories** — i.e. every grouping in the UI (the menu bar dropdowns from CR-002, the template sidebar) can be collapsed/expanded by the user, and groups can contain nested subcategories for hierarchical organization of their items.

## Motivation

User: "to make the selection more tidy" — as actions and shape types grow (user templates FR3/CR-003, imports FR12), flat grouped lists become hard to scan; collapsible sections hide what isn't needed, subcategories keep long lists (especially shapes) organized.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR6 | "UI consists of a template sidebar + editor canvas (minimal)" | "UI consists of a top menu bar with dropdown menus that group actions by category, a template sidebar, and an editor canvas (minimal); **groups in the menu bar and in the sidebar are collapsible and can be organized into subcategories**" |
| NFR4 | "Low learning curve / minimal UI complexity: a new user can compose a first diagram without documentation" | unchanged — contextually supported: tidier selection directly serves it |

Note: CR-002 (pending) proposes its own FR6 rewording (menu bar); if both are accepted, **this CR's wording is the combined final state** and supersedes CR-002's §14 edit.

## Conflict check

- **ADR 008 (egui/eframe):** collapsible sections (`egui::collapsing_header`) and nested menus (`egui::menu` inside menu items) are standard egui widgets on all four targets — consistent, no contradiction.
- **CR-002 interaction:** this CR extends it — the grouped menus must exist first (dependency). If CR-002 were rejected, this CR's premise changes; flag at acceptance.
- **CR-005 interaction:** collapse state is a user preference → persisted in the application config file once CR-005 lands; until then session-only.
- **No model/format/pipeline change** — pure GUI structure.

## Impact analysis

- **Stories:** `gui-app` — menu bar groups become collapsible with nested submenus (e.g. *Shapes → Containers / Flowchart*); template sidebar items organized into collapsible categories/subcategories.
- **Assumptions (confirm at acceptance):** predefined shapes get a static default category set; user/imported templates land in a declared or default ("Other") category — if user-declared categories are wanted, the shape-template definition gains an optional `category` field (template-file extension, confirm at acceptance); collapse state defaults to "expanded" for first use.
- **Format compatibility:** no (unless the optional template `category` field is confirmed in scope).

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace the FR6 row with the proposed wording (supersedes CR-002's pending FR6 edit if that lands first — apply the combined text once).
- arc42 §8 (gui-app sub-modules): one-line update alongside CR-002's.
- No new ADR planned (standard egui widgets, no architectural choice).
- Skeleton story **S-030** on `doc/stories/BOARD.md`, Why cites CR-011; **depends on CR-002** (grouped menus first); collapse-state persistence pairs with CR-005.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-032** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

- **S-032** (commit `eff03c1`): sidebar groups render as `egui::CollapsingHeader`s (expanded by default on first use, toggle reported to the app) with subcategory nesting from the pure view model `sidebar::group_templates` — built-ins: box/ellipse → “Basic”, any other built-in → “Other”; collection templates → “Other”; a group with ≥ 2 subcategories renders each as a nested collapsing header, exactly one renders flat (no redundant level). Per-group/subcategory open-closed state persists in the CR-005 config store (`AppConfig.sidebar_expanded: BTreeMap<String, bool>`; missing key = expanded, so pre-feature files load fully expanded) — written on every toggle, seeded at startup. File ▾ nests `Output ▸` (Copy, Text .txt) and `Export ▸` (Export SVG…, Export PNG…) via in-menu `menu_button`s (egui renders those as hover-opened submenus with their own arrow glyph); Open/Save/Save As… stay top-level, item order within each group unchanged, every item reachable and functional (S-014/S-015 tier-0b flows now drive the submenus through the new `Gui::click_submenu_item` harness helper). 9 new sidebar tests + rewritten S-025 grouped-actions test; Material goldens regenerated (diff confined to the sidebar band). No model/format/pipeline/seam change (conflict check held).
