---
title: S-003 - ad-format-spec
description: full grammar spec for the .ad format family — diagram files, template + collection files, anchor markers, resolution rules (docs only)
scope: docs/format
status: backlog
updated: 2026-10-22
---

# S-003: .ad format family specification

## Why

[ADR 004](../adr/004-diagram-file-format.md) (proposed) lists open points that must be settled before the parser exists: statement syntax, shape IDs, label escaping, connector statements, extension mechanism; [ADR 007](../adr/007-user-defined-diagram-objects.md) defers template/collection file syntax and anchor-marker characters to the format chapter. FR4 (save/load) and QS7 (lossless round-trip) are unimplementable without a frozen grammar.

## Dependencies

- S-002 — the grammar is a serialization of the Diagram aggregate; field names follow its model

## Acceptance criteria

1. **AC1 — Diagram file grammar.** Concept doc `doc/arc42/12-concepts/diagram-file-format.md` (from template) defines: file extension, version header (major/minor + unknown-field tolerance rule), scale statement, shape statements with stable IDs, text statements, connector statements (anchor refs by ID), label escaping rules, comments. Every Diagram aggregate field has exactly one representation; no information can be lost (QS7). Check: doc exists, review against `diagram-model` API.
2. **AC2 — Template + collection file syntax.** Same or sibling concept docs define: extensions, template declaration (name, fixed-size grid of svgbob-alphabet characters), anchor marker character(s) and their placement rules, collection manifest, `collection/name` → type_id resolution rule (ADR 007). Check: doc text is unambiguous enough to implement from without further decisions.
3. **AC3 — Escape + edge cases settled.** Label escaping (quotes/backslashes/newlines), empty labels, overlapping objects in file order, connector to a shape's default anchor vs explicit anchor — all have written rules. Check: review checklist in the doc.
4. **AC4 — Docs wired up.** New pages added to `doc/arc42/12-concepts/index.md` and mkdocs nav; ADR 004 + ADR 007 follow-up pointers updated to link the spec (status stays `proposed` until S-005/S-017 implement). Check: `mkdocs build --strict` exits 0.
5. **AC5 — Concept docs for shape template + collection created** (ADR 007 follow-up), from the template, with Implementation sections marked "not yet implemented".

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
