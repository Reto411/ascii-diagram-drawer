---
title: CR-008 - selectable arrow tips on both ends + arrow-shaped anchor markers
description: each connector end can carry a selectable tip (none / hollow arrow / empty rhombus / filled rhombus), changed by pressing onto the tip; GUI anchor dots become small arrow symbols (svgbob-rendered asset) to signal "connecting adds an arrow"
scope: connectors (tips + GUI affordances)
status: proposed
priority: medium
updated: 2026-10-30
---

# CR-008: Selectable arrow tips on both ends + arrow-shaped anchor markers

## Requested change

Arrows shall allow **exchanging their tips at both sides** — the available tip styles are **hollow tip, no tip, rhombus tip (empty), and rhombus tip (full)** — changed by **pressing onto the tip of the arrow**. Additionally, the **connector dots** in the GUI (the anchor connection points) shall be a **small arrow symbol** instead of plain dots, to clarify that they mean "adding an arrow"; such symbols may be rendered via svgbob and stored as assets.

## Motivation

Usability: users need to control what each end of a connection *means* (e.g. one-way vs. two-way relations, association vs. dependency-style ends) — today only the target end has a fixed arrowhead and the source end is bare. And plain dots at anchor points don't communicate that dragging from them creates an arrow.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR9 | "Connectors (lines/arrows) can be drawn between/around shapes; they bind to shape anchors and stay attached when the bound shape moves or resizes" | "…and stay attached when the bound shape moves or resizes; each end of a connector can carry a selectable tip style — no tip, hollow arrow, empty rhombus, filled rhombus — changed by pressing onto the tip; connection points are shown in the GUI as small arrow symbols to indicate that connecting adds an arrow" |

> Interaction with CR-007 (also rewords FR9): order-independent — apply sequentially on acceptance, whichever FR9 wording is current at that time gains this sentence.

## Conflict check

- **ADR 013 v2 §4:** "Arrowhead at the target end … `>` / `<` for horizontal approach, `v` / `^` for vertical … The arrowhead is omitted when there is no room." A single fixed arrowhead at the target end only — contradicted by selectable tips on **both** ends. On acceptance: **ADR 013 version bump** (sequential with CR-007's bump — whichever lands first takes v2→v3, the other v3→v4).
- **ADR 001 v2 (svgbob input alphabet):** has arrow glyphs `< > ^ v V` but **no rhombus/diamond glyphs**. The hollow tip maps to the existing svgbob arrowhead; the empty/filled rhombus would be literal Unicode content glyphs (`◇`/`◆`) stamped like the current arrowhead ("content over its edges" — already the mechanism). If adopted: **ADR 001 → v3** (two Unicode content-only glyphs); if the implementation maps rhombi to existing alphabet characters instead, ADR 001 stays. Decided at acceptance/implementation.
- **S-013 (done):** anchor dots are the current GUI affordance (`canvas.rs` overlays) — replaced by an arrow symbol rendered via svgbob and stored as an asset (same in-app asset approach as CR-006's window buttons).
- **NFR3:** holds — tip styles are model data; rendering stays a pure function of the model.

## Impact analysis

- **Stories:** S-007 (done — connectors) rework: `ConnectorInstance` gains per-end tip style, routing stamps the chosen glyph at each end, `.ad` stores it. S-013 (done — GUI): press-on-tip gesture + anchor-dot → arrow-symbol asset.
- **Assumptions (confirm at acceptance):** defaults are unchanged — source end = no tip, target end = hollow arrow — so every existing diagram renders identically; pressing a tip cycles through the four styles (a small selection menu is an acceptable alternative, decided at implementation).
- **Format compatibility:** **yes** — `.ad` extension (optional per-end tip data); ADR 004 round-trip / NFR2 at stake. Files without tip data stay valid and render exactly as today.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace the FR9 row with the proposed wording (sequentially after CR-007 if that lands first).
- **ADR 013 version bump** (sequential with CR-007); **ADR 001 → v3** only if rhombus tips use literal `◇`/`◆` content glyphs.
- Concept C9 (connector routing) updated; one line in the diagram-file-format concept for tip data.
- Skeleton story **S-027** on `doc/stories/BOARD.md`, Why cites CR-008; no hard dependencies (connectors exist since S-007).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** proposed
- **Date:** 
- **Reason / notes:** 

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
