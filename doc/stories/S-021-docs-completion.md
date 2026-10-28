---
title: S-021-docs-completion
description: fill remaining TBD arc42 chapters (§4, §7, §10, §11, §13, §15, §16) and close the overview tracker; docs-only
scope: docs
status: backlog
updated: 2026-10-22
---

# S-021: documentation completion

## Why

The arc42 set is the single source of truth (AGENTS.md); several chapters are still guidance/TBD (§4 stakeholders, §7 solution strategy, §10 deployment, §11 cross-cutting, §13 building blocks detail, §15 risks, §16 glossary). The completion phase requires the tracker in `overview.md` to reflect final state.

## Dependencies

- S-019 — deployment/web facts must be real before §10 is written
- S-020 — measured performance numbers feed §6/§15

## Acceptance criteria

1. **AC1 — Chapters filled.** §4, §7, §10, §11, §13, §15, §16 contain real content (no leftover "TBD — fill in" guidance notes): stakeholders table, technology choices with ADR links, deployment for native + web (incl. S-019 fallbacks), cross-cutting concerns per §11's list, per-crate building-block detail matching the implemented code, risks updated with measured data, glossary entries for all concept terms. Check: grep for `TBD`/`fill in` in those files returns nothing; content cross-checked against code.
2. **AC2 — Concept index complete.** §12 index lists every concept doc (incl. cell-grid, diagram-file-format, shape-template, collection) with one-line pointers. Check: index matches folder contents.
3. **AC3 — Tracker closed.** `doc/arc42/overview.md` progress tracker shows all chapters done; no chapter marked TBD. Check: file review.
4. **AC4 — Site builds strict.** `mkdocs build --strict` exits 0 with all pages in the nav (no orphan pages). Check: command output.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
