---
title: CR-001 - native file dialogs for open, save and export
description: Open/Save/Save As use a system file dialog (Save As can write the .ad document or the rendered .txt); Export becomes one system dialog with SVG/PNG format choice
scope: gui (file I/O + export UX)
status: accepted
priority: medium
updated: 2026-10-31
---

# CR-001: Native file dialogs for Open, Save and Export

## Requested change

The **Open**, **Save** and **Save As** toolbar buttons shall open a system file dialog. **Save As** additionally offers a choice of what to save: the diagram document (`.ad`) or the rendered text output (`.txt`). The **Export** action shall open one system dialog in which the export format (SVG or PNG) can be selected — replacing today's two separate "Export SVG…" / "Export PNG…" buttons and their inline path prompts.

## Motivation

User feedback while using the GUI: the inline path prompt (typed file path + OK) is clunky compared to the standard system dialog, and the toolbar carries redundant output actions ("Text .txt" beside Save/Save As, two Export buttons). Consolidating: a document has one canonical save target (`.ad`), its rendered text is an export of it — so "save as text" belongs in Save As's format choice, and SVG/PNG belong behind one Export dialog with a format choice.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR2 | "Diagram is exported as plain monospace text (svgbob-compatible ASCII per [ADR 001](../adr/001-glyph-resolution-alphabet.md)), copy/paste-able anywhere: terminals, READMEs, chat, docs — and directly parseable by svgbob tooling" | "Diagram is exported as plain monospace text (svgbob-compatible ASCII per [ADR 001](../adr/001-glyph-resolution-alphabet.md)), copy/paste-able anywhere: terminals, READMEs, chat, docs — and directly parseable by svgbob tooling; the text output can be written to a `.txt` file via Save As in addition to clipboard copy" |
| FR4 | "Diagrams are saved to and loaded from the open plain-text file format (round-trip: reload → identical model)" | "Diagrams are saved to and loaded from the open plain-text file format via system file dialogs — Open / Save / Save As (round-trip: reload → identical model); Save As additionally offers writing the rendered text output as `.txt` instead of the diagram document" |
| FR5 | "Diagrams export as SVG (vector) and PNG (raster) in addition to text; vector output is derived from the resolved character grid via svgbob ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)), so exports always match the text representation exactly" | "Diagrams export as SVG (vector) and PNG (raster) in addition to text, chosen in a single Export dialog where the user selects the target format; vector output is derived from the resolved character grid via svgbob ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)), so exports always match the text representation exactly" |
| NFR1 | "One codebase runs on Windows, macOS, Linux **and** in the browser (WASM)" | unchanged — system dialogs exist only on the native targets; the browser build keeps today's inline prompt / download behavior for Open/Save/Export (target-specific mechanism, same one-codebase requirement) |

## Conflict check

- **S-014 platform decision note** (`doc/stories/S-014-gui-file-io-text-output.md`): "No native file dialog in v1 — Open/Save As use an inline path prompt that behaves identically on both targets (`rfd` is a candidate for a later story)." This CR realizes exactly that deferred option, **for the native targets**. The note's "no native file dialog in v1" statement is superseded for native; the web behavior it describes stays. Not an ADR — no version bump; the note gets a supersession marker at acceptance.
- **ADR 008** (consequences): "file dialogs/clipboard come from eframe native APIs or small crates (`rfd`, `arboard`) — each must be verified on all four targets during the first milestone." Consistent — this CR is the anticipated follow-through, no contradiction.
- **arc42 §8** (gui-app sub-modules): `io` is already described as "file dialogs / clipboard UX on top of `DiagramStore` — per-target native APIs". Consistent.
- **NFR1**: browser has no system file dialog → inline prompt remains the web mechanism; no requirement change, stated in the table above.

## Impact analysis

- **Stories:** S-014 (done — file I/O + text output) rework: Open/Save As become native dialogs, Save As gains the `.ad`/`.txt` format choice, the standalone "Text .txt" button is removed (its function folded into Save As). S-015 (done — export + scale) rework: "Export SVG…"/"Export PNG…" collapse into one Export dialog with SVG/PNG selection. S-019 (backlog — wasm milestone): the web path must keep working with inline prompts/downloads; no regression allowed.
- **Format compatibility:** no — ADR 004's `.ad` round-trip and NFR2 are untouched; only how file paths are chosen changes, not what is written.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: replace FR2/FR4/FR5 rows with the proposed wording above.
- S-014 story file: mark the "No native file dialog in v1" note superseded by CR-001 (native targets).
- No new ADR planned — the dialog mechanism (e.g. `rfd` vs egui's built-in file picker) is an implementation detail decided at handoff by the cr-implementation skill; a new ADR only if that choice proves architecturally significant.
- Skeleton story **S-020** on `doc/stories/BOARD.md`, Why cites CR-001; no new dependencies (S-014/S-015 done); must not regress the S-019 web path.

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-039** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board. The S-014 "no native file dialog in v1" note is superseded by this story (native targets).

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

-
