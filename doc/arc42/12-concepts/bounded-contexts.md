---
title: bounded-contexts
description: the strategic DDD decomposition — five bounded contexts + application/presentation layer, realized as crate boundaries in one binary; context mapping defines how they talk
scope: architecture
status: stable
updated: 2026-10-22
---

# Concept: Bounded Contexts

## Definition

The system is decomposed into **bounded contexts** — each owns its own domain model and language, and the rules for communication between them are explicit (context mapping). In this project, bounded contexts are **crate boundaries inside a single binary** (native + wasm), not distributed services.

## Why it exists

[ADR 011](../../adr/011-domain-driven-design.md) adopted Domain-Driven Design for the building block decomposition. Without context boundaries, model concepts (e.g. "anchor", "shape") would silently mean different things in different crates and cross-crate coupling would be governed by convenience instead of rules. Contexts give each crate a stable ubiquitous language and make the *why* of every dependency edge reviewable.

## How it works / behaves

Five bounded contexts plus the (non-domain) application & presentation layer — full detail in [§8.2](../../arc42/08-building-block-view.md):

| Context | Crate(s) | Owns |
|---------|----------|------|
| **Diagram** (core domain) | `diagram-model` | the [[diagram-aggregate\|Diagram aggregate]], cell geometry, anchors, invariants |
| **Shape Catalog** | `shape-registry` | `ShapeType`, `Collection`, glyph patterns, anchor sets, resize rules |
| **Rendering & Export** | `render-pipeline`, `exporters` | `ResolvedGrid`, glyph resolution, text/SVG/PNG emission |
| **Persistence** | `ad-format` | `.ad` syntax — a *serialization* of the Diagram aggregate, not a second model |
| **Import** (v1+) | `ascii-import` | segmentation + template matching — an anti-corruption layer around foreign ASCII |
| **Application & Presentation** | `ui-api`, `core-services`, `gui-app` | application services (the ADR 009 seam), composition root, egui views |

Context mapping (how they talk):

- **Catalog → Diagram:** Customer/Supplier — the diagram carries only `type_id`s; meaning is resolved through the catalog at render time.
- **Diagram → Rendering & Export:** Open Host Service with a shared kernel — the renderer consumes the aggregate's value objects directly; cell geometry is the shared language.
- **Persistence ↔ Diagram:** Repository — `.ad` load/save (lossless round-trip, QS7).
- **Import → Diagram:** Anti-Corruption Layer — foreign ASCII is translated into our model; unknown regions are quarantined as reduced objects + warnings (FR12).

Rules: strictly one-directional dependencies (presentation → application layer → contexts); no cycles; cross-context communication via value types and published interfaces only; platform IO stays in the application layer.

## Relations

- **[[diagram-aggregate]]** — the model owned by the core Diagram context
- **Render pipeline** — hosted in the Rendering & Export context (link: `doc/arc42/12-concepts/render-pipeline.md`)
- **Glyph resolution** — stage 2 of the pipeline, same context (link: `doc/arc42/12-concepts/glyph-resolution.md`)

## Implementation

Scaffolded in S-001: all nine workspace crates exist per [§8.3](../../arc42/08-building-block-view.md) (one crate per context; Rendering & Export and Application/Presentation span two/three crates), each with `[lints] workspace = true`.

The context boundaries are **machine-checked** by the T5 dependency-audit test (`crates/core-services/tests/dependency_audit.rs`): it parses every member `Cargo.toml` and fails the build if any workspace-internal dependency edge is not on the §8.3 allow-list (which is acyclic, so the whole graph is acyclic). Adding a new cross-context edge requires extending the allow-list deliberately — that is the enforcement of the context-mapping rules above.
