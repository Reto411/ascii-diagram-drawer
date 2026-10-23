---
title: arc42-overview
description: arc42 document header, usage notes, and progress tracker for all 16 chapters
scope: architecture
status: draft
updated: 2026-10-23
---

# arc42 — ascii-diagram-drawer (Overview)

| | |
|---|---|
| **Document status** | Draft — being populated iteratively |
| **Version** | 0.5 (S-001 scaffold + S-002 `diagram-model` + S-003 `.ad` format family spec C6/C7/C8) |
| **Date** | 2026-10-23 |
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
| 5 | Constraints | [05](05-constraints.md) | 🟨 drafted (2026-10-18) — legal/boundary/framework conditions from existing decisions; performance budgets *proposed, pending confirmation*; GUI framework decided → [ADR 008](../adr/008-gui-framework-choice.md) (egui/eframe, `proposed`; Slint fallback); all Clippy lints deny → [ADR 010](../adr/010-deny-all-clippy-lints.md) (**accepted** v3 — implemented in S-001, extended in S-002: all groups deny at priority -1, ten documented exceptions) |
| 6 | Quality Model | [06](06-quality-model.md) | 🟨 drafted (2026-10-18) — quality tree, 13 measurable scenarios (usage + change, official short form), test goals T1–T7 with coverage gate ≥80% and golden-file determinism as CI invariant; perf scenarios await §5.3 budget confirmation |
| 7 | Solution Strategy | [07](07-solution-strategy.md) | ⬜ |
| 8 | Building Block View | [08](08-building-block-view.md) | 🟨 reworked (2026-10-22) with **DDD** per [ADR 011](../adr/011-domain-driven-design.md) (**accepted**) — subdomain analysis; 5 bounded contexts as crate boundaries (Diagram core, Shape Catalog, Rendering & Export, Persistence, Import v1+) + application/presentation layer; context mapping (Customer/Supplier, OHS/shared kernel, Repository, ACL); tactical patterns (aggregates, value objects, domain service, factory); CQRS/eventing explicitly out (§8.6); ADR 009 seam = application-layer boundary; WYSIWYG canvas decision; L3 zoom on glyph-resolve stage; **§8.3 layout scaffolded in S-001** (9 crates, dependency DAG machine-checked by the T5 audit test in `core-services`); open items: PNG rasterizer backend, trait signatures at scaffolding, default anchor sets (template file naming settled in S-003 → C7/C8); **§8.4.1 refined in S-002** (invariant split — anchor-geometry check is render-time, needs the catalog; `ShapeInstance.label` per ADR 004) |
| 9 | Runtime View | [09](09-runtime-view.md) | 🟨 drafted (2026-10-22) — responsive-UI threading per [ADR 012](../adr/012-responsive-ui-threading.md) v2 (`proposed`): dedicated UI thread never blocked by app work — frame-critical inline, PNG rasterization on worker thread (native) / inline (wasm); no separate process (impossible on web target), no async runtime; communication = direct calls on the §8.3 DAG + one job/result channel in core-services; scenarios RS1–RS6 as sequence diagrams (start, open, drag hot path P2, export incl. PNG worker path, save, ASCII import v1+); state: one aggregate + transient view state, grid derived never stored; error flows = synchronous Results (worker via channel) → non-blocking notices |
| 10 | Deployment View | [10](10-deployment-view.md) | ⬜ |
| 11 | Cross-cutting Concerns | [11](11-cross-cutting-concerns.md) | ⬜ |
| 12 | Concepts | [12](12-concepts/index.md) | 🟨 (C1–C8; C3/C4 with the DDD rework ADR 011, C5 cell grid with S-002, C6/C7/C8 format family with S-003) |
| 13 | Building Blocks (detail) | [13](13-building-blocks-detail.md) | ⬜ |
| 14 | Identification of Important Requirements | [14](14-important-requirements.md) | 🟨 (FR/NFR refined, Q-linked) |
| 15 | Risks and Technical Risks | [15](15-risks.md) | ⬜ |
| 16 | Glossary | [16](16-glossary.md) | ⬜ |

> **Implementation progress:** story-driven — see the board at [`doc/stories/BOARD.md`](../stories/BOARD.md). As of 2026-10-23: S-001 (workspace scaffold + quality tooling), S-002 (`diagram-model` aggregate, 100% line coverage) and S-003 (`.ad` format family spec — concept docs C6/C7/C8; grammar, escaping, versioning, resolution rules all settled) done; next in dependency order: S-004 (`ad-format` parser + canonical serializer) or S-005 (shape registry).
