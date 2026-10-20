---
title: arc42-overview
description: arc42 document header, usage notes, and progress tracker for all 16 chapters
scope: architecture
status: draft
updated: 2026-10-18
---

# arc42 — ascii-diagram-drawer (Overview)

| | |
|---|---|
| **Document status** | Draft — being populated iteratively |
| **Version** | 0.2 (split into per-chapter files, see [ADR 002](../adr/002-mkdocs-documentation-site.md)) |
| **Date** | 2026-10-18 |
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
| 5 | Constraints | [05](05-constraints.md) | 🟨 drafted (2026-10-18) — legal/boundary/framework conditions from existing decisions; performance budgets *proposed, pending confirmation*; GUI framework choice deliberately left open → [ADR 008](../adr/008-gui-framework-choice.md), under joint evaluation |
| 6 | Quality Model | [06](06-quality-model.md) | ⬜ |
| 7 | Solution Strategy | [07](07-solution-strategy.md) | ⬜ |
| 8 | Building Block View | [08](08-building-block-view.md) | ⬜ |
| 9 | Runtime View | [09](09-runtime-view.md) | ⬜ |
| 10 | Deployment View | [10](10-deployment-view.md) | ⬜ |
| 11 | Cross-cutting Concerns | [11](11-cross-cutting-concerns.md) | ⬜ |
| 12 | Concepts | [12](12-concepts/index.md) | 🟨 (C1+C2) |
| 13 | Building Blocks (detail) | [13](13-building-blocks-detail.md) | ⬜ |
| 14 | Identification of Important Requirements | [14](14-important-requirements.md) | 🟨 (FR/NFR refined, Q-linked) |
| 15 | Risks and Technical Risks | [15](15-risks.md) | ⬜ |
| 16 | Glossary | [16](16-glossary.md) | ⬜ |
