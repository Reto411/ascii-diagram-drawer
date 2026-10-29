---
title: arc42-overview
description: arc42 document header, usage notes, and progress tracker for all 16 chapters
scope: architecture
status: draft
updated: 2026-10-28
---

# arc42 — ascii-diagram-drawer (Overview)

| | |
|---|---|
| **Document status** | Draft — being populated iteratively |
| **Version** | 0.9 (S-001 scaffold + S-002 `diagram-model` + S-003 `.ad` format family spec C6/C7/C8 + S-004 `ad-format` parser/serializer + S-005 `shape-registry` catalog + S-006 render-pipeline place→resolve grid stage + S-007 anchor-bound connectors, ADR 013 + S-008 exporters text/SVG via svgbob, ADR 005 + S-009 PNG rasterization, ADR 016 + S-010 service seam `ui-api`/`core-services`, ADR 009 + S-011 responsive PNG worker, ADR 012 + S-012 eframe scaffold + WYSIWYG canvas spike, ADR 008 + S-013 editing gestures: move/resize/connect/select + S-014 file open/save + text output, ADR 009 v4 + S-017 template editor: text art + graphical anchors, ADR 007 accepted, ADR 009 v6) |
| **Date** | 2026-10-26 |
| **Target audience** | Developers, reviewers |

> **How to use this documentation:** Each chapter is its own file in this folder and contains italic guidance text (what belongs here) and `[TBD]` placeholders. Fill chapters incrementally; keep the progress table below updated. Remove guidance notes as chapters are finalized.

> **Relation to the official arc42 template:** this documentation follows a *tailored* 16-chapter layout (arc42 is explicitly tailorable). Mapping to the official 12 sections ([arc42.org/overview](https://arc42.org/overview/)):
>
> | Official section | Here |
> |---|---|
> | 01 Introduction and Goals | §1 (+ §3, §4) |
> | 02 Constraints | §5 |
> | 03 Context and Scope | §3 |
> | 04 Solution Strategy | §7 |
> | 05–07 Building Block / Runtime / Deployment View | §8–§10 |
> | 08 Crosscutting Concepts | §11, §12 |
> | 09 Architectural Decisions | ADRs in `doc/adr/` |
> | 10 Quality Requirements | §6 |
> | 11 Risks and Technical Debt | §15 |
> | 12 Glossary | §16 |
>
> Local chapters without an official counterpart: §2 Architecture Vision, §13 Building Blocks (detail), §14 Important Requirements.

## Progress tracker

| § | Chapter | File | Status |
|---|---------|------|--------|
| 1 | Introduction and Context | [01](01-introduction-and-context.md) | 🟨 (1.1+1.2 draft, design refs) |
| 2 | Architecture Vision | [02](02-architecture-vision.md) | 🟨 (drafted from existing ADRs/concepts, pending review) |
| 3 | Business Scope | [03](03-business-scope.md) | 🟨 (drafted from product interview) |
| 4 | Stakeholders and Users | [04](04-stakeholders-and-users.md) | ⬜ |
| 5 | Constraints | [05](05-constraints.md) | 🟨 drafted (2026-10-18) — legal/boundary/framework conditions from existing decisions; performance budgets *proposed, pending confirmation*; GUI framework decided → [ADR 008](../adr/008-gui-framework-choice.md) (**accepted** v3 — egui/eframe, first GUI milestone S-012 implemented on eframe 0.37; Slint fallback not needed); Clippy policy → [ADR 010](../adr/010-deny-all-clippy-lints.md) (**accepted** v4 — reduced from all-groups-deny to `clippy::all` deny: default-group violations fail the build, other groups not enforced) |
| 6 | Quality Model | [06](06-quality-model.md) | 🟨 drafted (2026-10-18) — quality tree, 13 measurable scenarios (usage + change, official short form), test goals T1–T7 with coverage gate ≥80% and golden-file determinism as CI invariant; perf scenarios await §5.3 budget confirmation |
| 7 | Solution Strategy | [07](07-solution-strategy.md) | ⬜ |
| 8 | Building Block View | [08](08-building-block-view.md) | 🟨 reworked (2026-10-22) with **DDD** per [ADR 011](../adr/011-domain-driven-design.md) (**accepted**) — subdomain analysis; 5 bounded contexts as crate boundaries (Diagram core, Shape Catalog, Rendering & Export, Persistence, Import v1+) + application/presentation layer; context mapping (Customer/Supplier, OHS/shared kernel, Repository, ACL); tactical patterns (aggregates, value objects, domain service, factory); CQRS/eventing explicitly out (§8.6); ADR 009 seam = application-layer boundary; WYSIWYG canvas decision; L3 zoom on glyph-resolve stage; **§8.3 layout scaffolded in S-001** (9 crates, dependency DAG machine-checked by the T5 audit test in `core-services`); open items resolved: PNG rasterizer backend → [ADR 016](../adr/016-png-rasterizer.md) (S-009), trait signatures pinned in S-010 ([ADR 009](../adr/009-ui-api-service-layer.md) v3 — application layer implemented: `ui-api` traits + `core-services` backends/composition root), default anchor sets settled in S-003 → C7/C8; **§8.4.1 refined in S-002** (invariant split — anchor-geometry check is render-time, needs the catalog; `ShapeInstance.label` per ADR 004) |
| 9 | Runtime View | [09](09-runtime-view.md) | 🟨 drafted (2026-10-22) — responsive-UI threading per [ADR 012](../adr/012-responsive-ui-threading.md) v2 (`proposed`): dedicated UI thread never blocked by app work — frame-critical inline, PNG rasterization on worker thread (native) / inline (wasm); no separate process (impossible on web target), no async runtime; communication = direct calls on the §8.3 DAG + one job/result channel in core-services; scenarios RS1–RS6 as sequence diagrams (start, open, drag hot path P2, export incl. PNG worker path, save, ASCII import v1+); state: one aggregate + transient view state, grid derived never stored; error flows = synchronous Results (worker via channel) → non-blocking notices |
| 10 | Deployment View | [10](10-deployment-view.md) | ⬜ |
| 11 | Cross-cutting Concerns | [11](11-cross-cutting-concerns.md) | ⬜ |
| 12 | Concepts | [12](12-concepts/index.md) | 🟨 (C1–C9; C3/C4 with the DDD rework ADR 011, C5 cell grid with S-002, C6/C7/C8 format family with S-003; **C1 glyph-resolution rule table + fallbacks F1–F5 finalized in S-006**, C2 render-pipeline Implementation filled in; **C9 connector routing added in S-007** (ADR 013)) |
| 13 | Building Blocks (detail) | [13](13-building-blocks-detail.md) | ⬜ |
| 14 | Identification of Important Requirements | [14](14-important-requirements.md) | 🟨 (FR/NFR refined, Q-linked) |
| 15 | Risks and Technical Risks | [15](15-risks.md) | ⬜ |
| 16 | Glossary | [16](16-glossary.md) | ⬜ |

> **Implementation progress:** story-driven — see the board at [`doc/stories/BOARD.md`](../stories/BOARD.md). As of 2026-10-29: S-001 (workspace scaffold + quality tooling), S-002 (`diagram-model` aggregate, 100% line coverage), S-003 (`.ad` format family spec — concept docs C6/C7/C8; grammar, escaping, versioning, resolution rules all settled), S-004 (`ad-format` parser + canonical serializer — line/column errors, golden files, proptest lossless round-trip, 86% coverage; ADR 004 accepted) and S-005 (`shape-registry` — built-in catalog as data, `.adc` collection loading per C7/C8, disjoint `collection/name` resolution) and S-006 (render-pipeline place→resolve → `ResolvedGrid` for shapes + text; C1 rule table finalized with fallbacks F1–F5, golden grids, proptest determinism + svgbob-alphabet purity, scale expansion per FR8; ADR 001 accepted) and S-007 (anchor-bound connectors: endpoints re-resolve from anchor references, straight/single-corner L routing of structural edges + arrowhead at the target end, five connector golden grids; ADR 013, concept C9) and S-008 (exporters — `to_text` with the written trailing-whitespace rule + `to_svg` passing the exact text bytes to `svgbob::to_svg`, byte-exact goldens for both media incl. empty grid and connector arrowheads; ADR 005 accepted) done), and S-009 (exporters — `to_png` rasterizes the exact SVG output via resvg behind a default `raster` feature, typed error seams incl. `Unsupported`, wasm32-compatible; ADR 016 accepted) and S-010 (service seam — `ui-api` four traits + seam errors/read models, `core-services` backends over every context crate, `Services::native()/web()` composition root, public fakes establishing the T7 pattern; ADR 009 accepted v3) and S-011 (responsive PNG worker — `PngWorker` in `core-services`: dedicated worker thread + bounded queue on native, inline executor on wasm32, per-frame polling job API with typed overflow rejection; ADR 012 accepted) and S-012 (eframe scaffold + WYSIWYG canvas spike — eframe/egui 0.37 app: model-space canvas drawing the exact resolved-grid bytes at measured glyph advance, template sidebar, click-to-place + drag with live re-render; pure gesture/mapping logic headless-tested; ADR 008 accepted v3) and S-013 (editing gestures — move/resize/connect/select over a pure gesture machine: press = grab, grid-aligned incremental moves, drop-on-anchor connectors, Esc/right-click cancel priority, Delete cascades through the aggregate; resize handle built-ins only per ADR 007 v3; GUI dots mirror the pipeline's anchor mapping) and S-014 (file open/save + text output — pure `fileio.rs` flows over the seam: parse-error notices with line/column keeping the previous model, save-failure surfacing, copy/save of the exact canvas bytes, and a confirm-before-overwrite dialog when a load skipped forward-compat statements; seam widened per ADR 009 v4 so `load` reports `skipped_unknowns`) and S-016 (collections sidebar + custom shape placement — seam read model widened per ADR 009 v5: `ShapeInfo` carries collection group + glyph preview; startup discovery of `*.adc` files from a configured directory in `core-services` with skip-and-warn, never fatal; pure sidebar view model grouped built-ins-first with monospace previews; custom shapes place at fixed pattern size and stamp verbatim with live anchors) and S-017 (template editor — the last v1 piece of ADR 007, now **accepted**: pure draft state in `gui-app template_editor.rs` — svgbob-alphabet cell validation with inline rejection hint, click-to-toggle anchors with standard edge names, at-least-one-anchor validation that blocks save; seam write path per ADR 009 v6 (`NewTemplate`, `CatalogError`, `ShapeCatalog::register`, `Renderer::preview_pattern` — live preview byte-equal to the saved template's render); one shared registry in `core-services registry_state.rs` so a save is listed and resolvable immediately, persisted as canonical `.adc` via new `shape-registry serializer.rs`) and S-015 (SVG/PNG export + scale control — pure `gui-app exportio.rs`: SVG = exact `to_svg` bytes to the sink; PNG through the ADR 012 worker with a per-frame-ticked “exporting…” state and no rasterization on the frame path; top-right scale control mutating model-level `diagram.scale()` so canvas + all exports follow, clamped [1, 8]; seam wiring per ADR 009 v7 — `ui-api` re-exports `ResolvedGrid`/`RenderError`, composition root exposes `Services.png_worker` via target-aware `PngWorker::for_target()`) done; next in dependency order: S-018 (free text), then S-019 (wasm milestone — needs it).
