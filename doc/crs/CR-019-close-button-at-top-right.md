---
title: CR-019 - close-button-at-top-right
description: the window close button sits at the left end of the app-bar control group; it shall be the rightmost control (top-right corner), per KDE/Windows convention
scope: gui
status: implemented
priority: low
updated: 2026-11-01
---

# CR-019: Window close button at the top right (rightmost of the app-bar controls)

## Requested change

On native targets, the three window controls in the custom Material app bar shall be ordered so that **close is the rightmost** — i.e. at the window's top-right corner. Left-to-right order: minimize, fullscreen, close.

Current behavior (verified by pixel scan of a HEAD render, 2026-11-01): the control *group* is already right-aligned in the app bar (8 pt margin from the window's right edge), but inside the group the order is **close ×, fullscreen □, minimize ─** — so close sits at the *left* end of the trio and minimize occupies the top-right corner.

## Motivation

User feedback (2026-11-01): "It is unusual to have it [the close button] at the left most." On KDE Plasma — the reporter's environment (see B-004) — as on Windows, the platform convention places close at the right end of the window controls; a close button at the left end of the group deviates from every app the user is used to and invites mis-clicks on minimize.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR14 | "The GUI is styled according to Material Design (color system, typography, elevation); on native targets the window border is a custom Material app bar with minimize, fullscreen and close controls — the editor canvas keeps its monospace rendering so the ASCII art stays WYSIWYG" | "The GUI is styled according to Material Design (color system, typography, elevation); on native targets the window border is a custom Material app bar with minimize, fullscreen and close controls laid out left-to-right in that order (close at the top-right corner) — the editor canvas keeps its monospace rendering so the ASCII art stays WYSIWYG" |

## Conflict check

- None. [ADR 017](../adr/017-material-design-title-bar.md) specifies the *set* of controls ("minimize, fullscreen toggle, close") but not their visual order; S-027's AC3 pins commands and glyphs, not placement. If accepted, ADR 017 records the order (version bump + history entry).

## Impact analysis

- **Stories:** S-027 (done) — its app bar (`gui-app/src/appbar.rs`, `window_controls`) is what changes: the visual order of the three buttons flips. AC3's command/glyph assertions stay valid; the implementing story pins the new order (tier-0b + goldens). No other stories affected.
- **Format compatibility:** none — GUI chrome only; `.ad`/`.adc`, rendering and exports untouched (ADR 004 / NFR2 not at stake).

## Decision plan

- On acceptance, in one commit: §14 FR14 wording update; ADR 017 version bump recording the control order; skeleton story S-040 on the board (ACs written at implementation).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-11-01
- **Reason / notes:** blanket acceptance by the user — "Fix all bugs, implement all change requests and do extensive regression testing including End to end tests" (2026-11-01). Applied: §14 FR14 updated, ADR 017 → v2, skeleton story S-040 on the board.

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

Landed as **S-040** (2026-11-02): `window_controls` in `gui-app/src/appbar.rs` now adds the three buttons in reverse code order (close first) inside its `Layout::right_to_left` scope, so the visual order is minimize ─ → fullscreen □ → close × with close at the top-right corner; commands and glyphs unchanged. Pinned by new tier-0b test `window_controls_render_minimize_fullscreen_close_left_to_right` + feature-bar decomposition `ui_decomposition_window_controls`; goldens regenerated (diff confined to the control glyph row).
