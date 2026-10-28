---
title: S-003 - ad-format-spec
description: full grammar spec for the .ad format family — diagram files, template + collection files, anchor markers, resolution rules (docs only)
scope: docs/format
status: done
updated: 2026-10-23
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

- Grammar settled as three concept docs from the template: `diagram-file-format.md` (C6, normative `.ad` grammar), `shape-template.md` (C7, template block syntax + anchor declarations), `collection.md` (C8, `.adc` file + resolution rule). All cross-linked; C6/C7/C8 added to §12 index and mkdocs nav.
- Key decisions recorded in the spec: statements are single physical lines (newlines inside labels/text are `\n` escapes); only `\"`, `\\`, `\\n` are valid escapes; canonical form = header, always-emitted `scale`, objects in stored order, single spaces, LF, no leading zeros; versioning = major reject / minor skip-and-preserve-verbatim (QS7 across minor upgrades); ids/anchors split at the LAST dot.
- ADR 007's open "anchor marker character" point settled by **coordinate-based** `anchor <name> at <x> <y>` declarations — no marker character exists, so nothing can collide with the svgbob alphabet. Recorded in ADR 007 v4 history; decision unchanged, status stays `proposed`.
- Collection identity = filename stem (no in-file name statement → no drift); the `.adc` file itself is the manifest; bare type-ids = built-ins only, `collection/name` = user collections, disjoint namespaces, no shadowing.

## Verification

- [x] AC1 — `diagram-file-format.md` exists (from template): extension, `ad <major>.<minor>` header + tolerance rule, `scale`, `shape ... as <id>`, `text`, `connect ... from <id>.<anchor> to <id>.<anchor> [dashed]`, escaping rules, `#` comments. Field-mapping table covers every `diagram-model` field exactly once (checked against the S-002 API: `ShapeInstance{id,label,pos,size,type_id}`, `TextInstance{id,pos,text}`, `ConnectorInstance{from,id,style,to}`, `Diagram.scale`, object order).
- [x] AC2 — sibling docs define `.adc` extension, template declaration (`template <name> <w>x<h>` + exactly h grid rows), anchor placement rules (bounds, unique names, shareable cells), collection manifest (= the file: header + ordered blocks), and the `collection/name` → type_id resolution rule. EBNF-level precision; no open decisions remain.
- [x] AC3 — "Edge cases (settled rules)" section: escapes (only 3 valid; raw newline impossible), empty labels (shape ok / text rejected by model), overlap = file order is z-order, connectors always name explicit anchors (no default anchor in v1), plus referential integrity at load, integer canonical form, comment/blank-line semantics.
- [x] AC4 — C6/C7/C8 in `index.md` + mkdocs nav; ADR 004 → v2 and ADR 007 → v4 with follow-up pointers linking the spec (both stay `proposed`). `mkdocs build --strict` exits 0, zero warnings.
- [x] AC5 — `shape-template.md` + `collection.md` created from the template; both Implementation sections marked "Not yet implemented" with the landing stories (S-005 parse, S-017 graphical editor). 
