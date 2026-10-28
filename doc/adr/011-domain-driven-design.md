---
title: "011 - domain-driven-design"
description: the building block decomposition is organized with Domain-Driven Design — subdomains, bounded contexts (as crate boundaries in one binary), context mapping, and a deliberately limited set of tactical patterns; no CQRS/eventing
scope: architecture
status: accepted
version: 1
updated: 2026-10-22
---

# ADR 011: Domain-Driven Design for the Building Block Decomposition

## Context

[§8 Building Block View](../arc42/08-building-block-view.md) was originally organized by *technical tier* (core crates / service seam / GUI). The requested direction: organize the decomposition with **Domain Driven Design** instead.

Constraints in play:

- This is a **single-binary desktop/browser tool** — one process, fully offline ([§5.2](../arc42/05-constraints.md)), single user. Enterprise-scale DDD (distributed contexts, event storming, CQRS) does not fit the scale.
- The domain *does* have clear substructure: diagram editing vs. rendering/export vs. shape catalog vs. persistence vs. import — and the existing crate boundaries already align with natural context boundaries.
- [ADR 009](009-ui-api-service-layer.md) (service seam) must stay valid: it solves real problems (per-target IO, PNG optionality, GUI testability) that DDD reframes but does not replace.

Decision forced: which level of DDD to adopt, and how it maps onto the existing crate/ADR structure.

## Decision

**Adopt Domain-Driven Design for the building block decomposition — strategic design in full, tactical patterns selectively.**

1. **Strategic design (full):** subdomain classification (core / supporting / generic); five bounded contexts — **Diagram** (core domain), **Shape Catalog**, **Rendering & Export**, **Persistence (.ad)**, **Import (v1+)** — plus the **Application & Presentation layer**. Context mapping defines how contexts communicate: Customer/Supplier (catalog → diagram), OHS with shared kernel (diagram → rendering), Repository (persistence serializes the Diagram aggregate), Anti-Corruption Layer (import translates foreign ASCII).
2. **Bounded contexts are crate boundaries inside one binary** — no distributed contexts, no network between them (consistent with §5.2 offline mandate and Q2 one-codebase portability).
3. **Tactical patterns (selected):** aggregate roots (`Diagram`; `ShapeType`/`Collection`), entities vs. value objects (identity-bearing objects; immutable cell geometry), invariants enforced at the aggregate boundary, a domain service for rendering (spans two contexts), Repository pattern for `.ad`, factory for import assembly.
4. **Deliberate non-adoption** (recorded in [§8.6](../arc42/08-building-block-view.md)): no CQRS, no event sourcing, no domain-event bus, no anemic model.
5. **[ADR 009](009-ui-api-service-layer.md) remains valid and unchanged.** Its `ui-api`/`core-services` seam is reinterpreted as the **application-layer boundary** of the DDD structure: application services behind traits, composition root per target, direct aggregate manipulation for diagram mutations.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Keep the technical-tier decomposition (status quo) | Boundaries justified by "pure vs. impure" rather than language ownership; no stable ubiquitous language; persistence and import stay ad-hoc attachments instead of first-class contexts |
| Full enterprise DDD (CQRS, event sourcing, distributed bounded contexts) | Single-user local tool in one process — massive over-engineering; would fight the determinism goal (Q1) and the offline/one-codebase constraints (§5.2, Q2) |
| Rename crates to DDD terms without documented boundaries | Vocabulary without substance: no context mapping, no rules for cross-context communication, no subdomain analysis — worse than the status quo because it looks structured while being arbitrary |

## Consequences

- **Positive:** stable ubiquitous language per context; each crate owns exactly one context's model; cross-context communication is explicit and rule-governed (context mapping); the ADR 009 seam gains a clear architectural identity (application layer) without any API change; testability story (T5/T7, Q5) unchanged.
- **Negative / costs:** more upfront design vocabulary in docs; risk of DDD ceremony creeping into implementation — guarded by the explicit non-adoption list ([§8.6](../arc42/08-building-block-view.md)); reviewers must know enough DDD to hold the line on "no anemic model" and "invariants at the aggregate boundary".
- **Follow-ups:** concept docs C3 (bounded contexts) and C4 (diagram aggregate) created alongside this ADR; §8 reworked accordingly; keep context boundaries 1:1 with crates even if scaffolding renames anything; revisit only if a genuinely second domain language appears (e.g. a real multi-user collaboration feature).

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v1 (2026-10-22):** initial decision — strategic DDD in full (subdomains, five bounded contexts as crate boundaries, context mapping), selective tactical patterns (aggregates, value objects, domain service, repository, factory); CQRS/eventing/anemic model explicitly out; ADR 009 reinterpreted as the application-layer boundary.
