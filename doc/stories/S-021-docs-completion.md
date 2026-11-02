---
title: S-021-docs-completion
description: fill remaining TBD arc42 chapters (§4, §7, §10, §11, §13, §15, §16) and close the overview tracker; docs-only
scope: docs
status: done
updated: 2026-11-02
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

- One `write`/`edit` pass per chapter, facts taken from the tree (per the 8da008c lesson): crate public APIs for §13, S-019/S-020 measurements for §15, ADR 008/015/016 resolutions for the stale-TBD fixes.
- Beyond the seven AC1 chapters, resolved stale TBDs that contradicted recorded decisions: §1.3/§1.4 (filled from §5/§4), §2 diagram labels (“framework TBD” → ADR 008 egui/eframe; “rasterize (TBD)” → ADR 016 resvg) + Q2 row (bundle budget → proposed P3, pending confirmation, R1 measured ≈ 12.5 MiB unoptimized), §8.4.3 PNG bullet (“Rasterizer TBD” → ADR 016 resvg 0.49 behind the optional `raster` feature), §14 requirements-management pointer (this chapter + board + CRs).
- overview.md header: document status Draft → Final (v1.0), version/date bumped; how-to note updated (chapters finalized); progress log gained the S-021 entry.

## Verification

- [x] AC1 — `grep "TBD\|fill in"` over §4/§7/§10/§11/§13/§15/§16 returns nothing (exit 1), and a whole-set scan for `[TBD]` markers across all arc42 chapters is also clean. Content cross-checked against code: §13 BB1–BB9 interfaces match the actual public APIs (`diagram-model` Diagram ops incl. `add_bend`/`move_bend`/`remove_bend`/`set_tip`; `shape-registry` `rasterize`/`min_size`/collection I/O; `render-pipeline` `render` + `connector_routes` + `bench_fixtures`; `exporters` text/SVG/PNG + `SvgOptions`; `ui-api` four traits + seam errors; `core-services` `Services::native()/native_with_collections()` + modules incl. `webio`/`worker`/`fakes`; `gui-app` module list matches `gui-app/src/`); §15 R2/R5 facts from the S-020 session (wgpu-hal E0277 under WASI p1; 26.6 s debug determinism run); T5 audit test confirmed at `core-services/tests/dependency_audit.rs`.
- [x] AC2 — §12 index lists all 10 concept docs with one-line purpose pointers; verified programmatically that every file in `doc/arc42/12-concepts/` (except `index.md`) is referenced by the index (10/10 OK).
- [x] AC3 — `overview.md` chapter table: 16/16 rows ✅, zero ⬜/🟨; header now “Final (v1)”, version 1.0, date 2026-11-02; progress log has the S-021 entry.
- [x] AC4 — `mkdocs build --strict` exits 0, no warnings, no orphan pages (all pages in nav). 
