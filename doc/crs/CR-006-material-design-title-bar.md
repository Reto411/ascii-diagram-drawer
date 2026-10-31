---
title: CR-006 - material design gui with custom title bar
description: style the whole GUI according to Material Design — custom (undecorated) window border/app-bar with minimize, fullscreen and close controls; button symbols may be ASCII art rendered by the app's own pipeline and stored as SVG assets
scope: gui (theming + window chrome)
status: accepted
priority: medium
updated: 2026-10-31
---

# CR-006: Material Design GUI with custom title bar

## Requested change

The **border of the window** shall be a Material Design app bar, and **the overall GUI shall be styled according to Material Design**. The window controls in the border shall consist of **hide (minimize), fullscreen, and close**. If symbols are needed for these buttons, they may be generated as ASCII art within the application and rendered/stored as assets (e.g. SVG).

## Motivation

User feedback: "I want to look the application modern." The current default-egui look with the OS window frame feels dated; a consistent Material Design surface — including an app-owned title bar — gives the tool a modern, cohesive appearance.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR14 | new — no current requirement covers GUI theming or window chrome | "The GUI is styled according to Material Design (color system, typography, elevation); on native targets the window border is a custom Material app bar with minimize, fullscreen and close controls — the editor canvas keeps its monospace rendering so the ASCII art stays WYSIWYG" |
| NFR1 | "One codebase runs on Windows, macOS, Linux **and** in the browser (WASM)" | unchanged — contextually affected: window controls exist only where a window exists; the browser target keeps the browser's own chrome and shows no fake minimize/fullscreen/close buttons |

## Conflict check

- **ADR 008 (egui/eframe):** eframe supports undecorated viewports + viewport commands (`StartDrag`, `Minimized`, `Maximized`, `Close`) on native — consistent; wasm has no window chrome, hence the target-specific note above. This is a genuinely new UI decision → **new ADR (`status: proposed`)** at acceptance (custom title bar + Material styling approach).
- **WYSIWYG invariant (S-012/S-015):** the canvas draws the exact `to_text` bytes in monospace — Material typography applies to chrome/panels/menus, **never to the canvas** (assumption, confirm at acceptance).
- **CR-002 / CR-004 / CR-005 interaction:** cross-cutting and order-independent — the menu bar, preview pane and Options menu are all restyled by this theme; no wording conflicts.
- No file-format impact; NFR3/NFR5 untouched (styling never touches the render pipeline).

## Impact analysis

- **Stories:** S-012 (done — scaffold) rework: undecorated viewport on native + custom app bar (drag/minimize/fullscreen/close) + Material `Style`/`Visuals` applied app-wide. All delivered GUI stories (S-013…S-018) are visually restyled — regression risk concentrated in the deferred tier-1/2 visual goldens (image rebuild pending). Icon assets: ASCII art authored in-app → rendered through the app's own svgbob pipeline (`to_svg`) → stored as SVG assets, rasterized via resvg for display (the user's suggested approach; zero new dependencies).
- **Scope note:** static Material styling (colors/typography/elevation) — animated ripples are not part of this CR (egui has no animation system); flag at acceptance.
- **Format compatibility:** no.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: append **FR14** with the proposed wording.
- New ADR (`status: proposed`): Material Design theme + custom title bar (undecorated native viewport, viewport commands, web keeps browser chrome, icon approach ASCII-art → svgbob SVG assets).
- arc42 §8 (gui-app sub-modules): one line for the theme/title-bar module.
- Skeleton story **S-025** on `doc/stories/BOARD.md`, Why cites CR-006; no hard dependencies (cross-cutting GUI restyle, ideally after CR-002/CR-004/CR-005 so it styles the final layout once).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-027** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board. New [ADR 017](../adr/017-material-design-title-bar.md) (`proposed` until S-027 implements it).

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
