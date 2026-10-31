---
title: CR-005 - options menu with theme selection and config persistence
description: an Options menu to modify application settings — starting with UI theme (light / dark / native-inherited, default) — persisted in the application's config file across sessions
scope: gui (options/settings + theming)
status: accepted
priority: low
updated: 2026-10-31
---

# CR-005: Options menu with theme selection and config persistence

## Requested change

The application shall have an **Options menu** where the user can modify application settings. It shall include a **theme** option with the choices **light** (mainly white), **dark** (mainly dark), and **native** (inherited from the window manager / system appearance — the default). Changes made in Options shall be **persisted in a config file of the application** and take effect across sessions.

## Motivation

User feedback: the UI is currently fixed to a dark theme with no way to adapt it to personal preference or the system's appearance; and as features grow, the application needs a standard home for user settings — the Options menu becomes that extension point (theme is its first option).

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR13 | new — no current requirement covers application options/theming | "The application provides an Options menu where user preferences can be modified; the UI theme is selectable — light (mainly white), dark (mainly dark), native (inherited from the system/window manager, default) — and the selection persists across sessions in the application's config file" |
| NFR4 | "Low learning curve / minimal UI complexity: a new user can compose a first diagram without documentation" | unchanged — contextually supported: personalization is one menu away, no documentation needed |

## Conflict check

- **S-012 implementation note** (`gui-app/src/main.rs`): "fixed dark theme" — `egui::Visuals::dark()` is hardcoded at startup. Superseded by the selectable theme; an implementation note, not an ADR.
- **ADR 008 (egui/eframe):** `Visuals::light()` / `Visuals::dark()` are standard egui widgets — consistent. **Native** inheritance has no built-in in eframe → best-effort per target, which the request explicitly allows ("optional if not feasible"): where system appearance cannot be detected (e.g. some desktop targets, or before detection on wasm), native falls back to dark. No architectural conflict.
- **NFR1:** config file + theming must work on all four targets; native detection is best-effort per target (browser may use `prefers-color-scheme`).
- **Interaction with CR-002** (menu bar): order-independent — the Options menu becomes a dropdown alongside File when that lands first, or a top-level menu otherwise.

## Impact analysis

- **Stories:** S-012 (done — scaffold, fixed dark visuals) rework: apply the selected theme at startup + live on change; new Options menu and config load/save in `gui-app` (small new module). No other stories affected.
- **Scope note:** this CR ships the Options *mechanism* with the theme option only; future settings extend the same menu/config without further requirement changes.
- **Format compatibility:** no — the application config file is separate from the diagram format (ADR 004 / NFR2 untouched).

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: append **FR13** with the proposed wording (new IDs appended, never renumbered).
- arc42 §8 (gui-app): one line noting user preferences persist in an application config file.
- No new ADR planned — egui `Visuals` are standard; config file location/format (e.g. JSON in the platform config dir) is an implementation detail recorded in the story.
- Skeleton story **S-024** on `doc/stories/BOARD.md`, Why cites CR-005; no hard dependencies.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-023** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
