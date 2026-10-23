---
title: collection
description: a named plain-text file (.adc) grouping shape templates; the manifest + resolution rule for collection/name type-ids; read before implementing collection loading (S-005)
scope: format / domain model
status: stable
updated: 2026-10-23
---

# Concept: Collection

## Definition

A collection is a named, open plain-text file (`.adc`, ADR 004 format family) that groups related [shape templates](shape-template.md) — the shareable unit of user-defined content ([ADR 007](../../adr/007-user-defined-diagram-objects.md)). One file per collection; sharing a collection means sharing one ordinary text file (FR3).

## Why it exists

Templates need a container with an identity that diagrams can reference stably (`collection/name`) and users can copy between machines. A single plain-text file keeps the whole feature dependency-free, diff-able, and reviewable — no database, no binary blobs.

## How it works / behaves

### File syntax (v1 = `adc 1.0`)

```ebnf
file    = "adc", int, ".", int, template-block* ; versioning + tolerance: same rules as .ad (see below)
```

- Header `adc <major>.<minor>` first; the major/minor compatibility and unknown-statement preservation rules of the [diagram file format](diagram-file-format.md) apply unchanged inside `.adc` files.
- The **collection file itself is the manifest**: header + its ordered `template` declarations. There is no separate index or registry — the set of `template` blocks *is* the collection's content, and declaration order is preserved on re-save (canonical form).
- **Collection identity = filename stem** (`mycol.adc` → collection `mycol`, case-sensitive). No in-file name statement — a single source of truth, so declared and actual names can never drift.

### Template blocks

Each template block follows the syntax in [shape-template](shape-template.md): declaration line, exactly `height` grid rows, then its `anchor` statements. Blocks are separated by nothing (blank lines between blocks are tolerated as decoration).

### Resolution rule (`collection/name` → type-id)

When a diagram statement references a shape `<type-id>`:

1. **Bare token** (no `/`) → resolved against the **built-in catalog only** (app-provided, S-005). A bare id can never resolve to a user template.
2. **`collection/name`** → resolved against user collections: load `<collection>.adc`, find a template named `name`. No such collection or template → load error (no silent fallback to built-ins).
3. No shadowing in either direction; the two namespaces are disjoint by construction.

The **search path** (where `.adc` files are looked up — e.g. project directory + user directory) is an *application* concern of the GUI/service layer, not part of the format spec.

### Loading semantics

- Collections are loaded on demand when a diagram references them (S-005); template definitions are immutable once parsed.
- A collection never references other collections (no includes in v1).
- Editing: text mode is the source of truth; the graphical editor (FR11, S-017) reads and writes the same file — it must re-serialize canonically so hand edits survive a round trip through the GUI.

## Relations

- **Shape template** — the content unit collections group (link: `doc/arc42/12-concepts/shape-template.md`)
- **Diagram file format** — shared versioning/tolerance scheme; `collection/name` type-ids appear in `.ad` shape statements (link: `doc/arc42/12-concepts/diagram-file-format.md`)
- **Bounded contexts** — collections belong to the Shape Catalog context (`shape-registry` crate), not to `diagram-model` (link: `doc/arc42/12-concepts/bounded-contexts.md`)

## Implementation

Not yet implemented — loading + validation lands in `crates/shape-registry` with S-005 (built-in catalog + template/collection loading); the GUI collections sidebar is S-016, the graphical editor S-017.
