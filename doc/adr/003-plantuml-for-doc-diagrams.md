---
title: 003 - plantuml-for-doc-diagrams
description: diagrams inside project documentation use PlantUML until ascii-diagram-drawer itself can produce them
scope: documentation / tooling
status: accepted
updated: 2026-10-17
supersedes: null
---

# ADR 003: PlantUML for diagrams in documentation

## Context

The arc42 chapters and concept docs need diagrams (building block views, runtime sequences, deployment topology — e.g. §7.2, §8.2, §10.2 currently hold placeholders). The project's own tool (ascii-diagram-drawer) does not exist yet, so diagrams must be produced with an external, text-based tool that fits the MkDocs site and stays diffable in git.

## Decision

- **PlantUML** is used for all diagrams in project documentation until ascii-diagram-drawer can generate its own output.
- Diagrams are written as PlantUML source (```plantuml code fences or `.puml` files) so they remain plain text, reviewable in diffs, and renderable by MkDocs tooling.
- When ascii-diagram-drawer reaches a usable state, a follow-up ADR will decide whether to migrate existing diagrams to the native format (and supersede this ADR).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Mermaid | Weaker for UML-style building block / sequence diagrams that arc42 §8/§9 need |
| Hand-drawn images (PNG/SVG) | Not diffable, not editable as text, rots quickly |
| Wait for ascii-diagram-drawer itself | The tool doesn't exist yet; documentation needs diagrams now |

## Consequences

- **Positive:** text-based, git-friendly diagrams; consistent look across the doc site
- **Negative / costs:** PlantUML rendering dependency for the MkDocs site (Java or a render plugin); migration decision deferred
- **Follow-ups:** wire PlantUML rendering into the MkDocs build; revisit when ascii-diagram-drawer is usable
