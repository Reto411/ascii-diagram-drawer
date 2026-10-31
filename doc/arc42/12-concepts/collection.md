---
title: collection
description: a named plain-text file (.adc) grouping shape templates; the manifest + resolution rule for collection/name type-ids; read before implementing collection loading (S-005)
scope: format / domain model
status: stable
updated: 2026-10-29 (S-017: editor save path)
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

Each template block follows the syntax in [shape-template](shape-template.md): declaration line, exactly `height` grid rows, then its `anchor` statements — and, from minor version 1.1 (CR batch accepted 2026-10-31), optional `resize` statements (C10) and `region` declarations (FR19). Blocks are separated by nothing (blank lines between blocks are tolerated as decoration).

### Resolution rule (`collection/name` → type-id)

When a diagram statement references a shape `<type-id>`:

1. **Bare token** (no `/`) → resolved against the **built-in catalog only** (app-provided, S-005). A bare id can never resolve to a user template.
2. **`collection/name`** → resolved against user collections: load `<collection>.adc`, find a template named `name`. No such collection or template → load error (no silent fallback to built-ins).
3. No shadowing in either direction; the two namespaces are disjoint by construction.

The **search path** (where `.adc` files are looked up — e.g. project directory + user directory) is an *application* concern of the GUI/service layer, not part of the format spec.

### Loading semantics

- Collections are loaded on demand when a diagram references them (S-005), and — separately, as an application concern — the native GUI discovers `*.adc` files from a configured directory at startup (`--collections <dir>`, S-016); invalid files become warnings, never startup failures. Template definitions are immutable once parsed.
- A collection never references other collections (no includes in v1).
- Editing: text mode is the source of truth; the graphical editor (FR11, S-017) reads and writes the same file — it re-serializes canonically (`shape-registry serializer.rs`), so hand edits to known statements survive a round trip through the GUI. v1 note: a GUI save rewrites the whole file from parsed data, so unknown statements of a *newer minor version* would not be preserved across such a save — no 1.x files exist yet (1.0 rejects them at parse), so nothing is lost today; preservation on collection saves is a follow-up when/if newer minors ship (same precedent as ADR 009 v4 for `.ad` documents).

## Relations

- **Shape template** — the content unit collections group (link: `doc/arc42/12-concepts/shape-template.md`)
- **Diagram file format** — shared versioning/tolerance scheme; `collection/name` type-ids appear in `.ad` shape statements (link: `doc/arc42/12-concepts/diagram-file-format.md`)
- **Bounded contexts** — collections belong to the Shape Catalog context (`shape-registry` crate), not to `diagram-model` (link: `doc/arc42/12-concepts/bounded-contexts.md`)

## Implementation

`crates/shape-registry` (landed with S-005): `templates.rs::parse_collection(name, text)` parses a `.adc` file — identity passed in from the filename stem per this spec — into a `Collection` plus ordered statement slots and the source header version, so unknown statements from higher minor versions are preserved verbatim and re-emitted legally on save; `registry.rs::ShapeRegistry` implements the resolution rule above (disjoint bare/qualified namespaces, typed errors). Startup discovery + the grouped sidebar with glyph previews landed with S-016: `core-services collections.rs` (directory scan → registry + warnings) and `gui-app sidebar.rs` (pure view model over the `ShapeCatalog` seam). CR-013 (accepted 2026-10-31, FR16) replaces those monospace glyph previews with **pre-rendered WYSIWYG previews** generated from the same pattern bytes the renderer stamps — landing in S-034; CR-012 adds a name filter input to the sidebar (S-033). The graphical editor's save path landed with S-017: `gui-app template_editor.rs` (draft state) → seam `ShapeCatalog::register` ([ADR 009](../../adr/009-ui-api-service-layer.md) v6) → `core-services registry_state.rs` (shared registry + canonical `.adc` persistence via `shape-registry serializer.rs`) — a saved template is listed by the sidebar and resolvable by the renderer immediately, no re-composition.
