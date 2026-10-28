---
title: 004 - diagram-file-format
description: diagram files use a custom, versioned, line-based plain-text format with explicit grid coordinates — not JSON/YAML and not rendered art
scope: format
status: accepted
updated: 2026-10-23
version: 3
---

# ADR 004: Open plain-text diagram file format

## Context

Diagrams must be persisted in a file format ([FR4](../arc42/14-important-requirements.md)): save/load round-trip (reload → identical model). The format is constrained by the project's core quality goals and motivation:

- **NFR2 / Q3 (open, readable):** the file stays human-readable *without any renderer* — "any text editor or `cat` shows the diagram" (§1 Background).
- **NFR3 / Q1 (deterministic, no auto-layout):** element positions are exactly where the user placed them. Unlike PlantUML/Graphviz, there is no layout engine that may rearrange the diagram — so the format must store explicit coordinates on the cell grid, and re-saving must be byte-stable.
- **NFR1 / Q2 (one codebase → desktop + WASM):** the parser/serializer runs in the platform-independent core crate ([NFR5](../arc42/14-important-requirements.md)), so it must stay lightweight and dependency-free.
- The format must encode at least: shape instances (type, label, grid position, size), diagram-level properties (e.g. scale per §1.1 Obj. 7 / FR8), and be extensible toward reusable shape collections (§1.1 Obj. 3) and connectors.

## Decision

Diagram files use a **custom, versioned, line-based plain-text format** (extension `.ad`): one statement per line, each describing one element **with explicit cell-grid coordinates**. It is a small command language in the spirit of PlantUML text — minus the layout engine: what you place is what the file says.

Key properties:

1. **Explicit coordinates.** Every shape instance stores its grid position and size; nothing is derived at load time.
2. **Versioned header.** A header line declares the format version so future extensions (collections, connectors) stay backward-compatible.
3. **Plain text, UTF-8, LF line endings; `#` comments.** No binary layers, no escaping of the structure itself.
4. **Canonical serialization (Q1/NFR3).** The writer emits a fixed statement order (diagram properties first, then elements in stored order) with stable spacing: `serialize(parse(s)) == s` and the same model always yields byte-identical files.

Illustrative v1 sketch *(superseded by the normative grammar — see [diagram-file-format](../arc42/12-concepts/diagram-file-format.md), settled in S-003; kept for history)*:

```
# my-diagram.ad (format version 1)
scale: 2

box    "User"     at 1,1   w 8 h 3
box    "Database" at 12,5  w 10 h 3
circle "Cache"    at 20,1  w 6 h 6
```

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| JSON | Structurally correct but not human-readable as a diagram; poor diffs for hand-editing; no comments — violates the spirit of NFR2/Q3 ("`cat` shows the diagram") |
| YAML / TOML | Readable as *data*, but verbose and structural (indentation/brackets): you see coordinates, not the diagram; adds a parser dependency; multiple valid serializations of the same data threaten canonical output (Q1) |
| Rendered ASCII art as source of truth | `cat` shows the picture literally, but a rendered grid cannot be uniquely decomposed back into shapes → breaks FR4 round-trip and NFR3 |
| Reuse an existing text DSL (PlantUML, Graphviz, Mermaid) | Their automatic layout engines contradict NFR3/Q1; none models a manually placed canvas with explicit coordinates; would tie us to someone else's format decisions |

## Consequences

- **Positive:** `cat`-readable (shapes, labels, and positions are visible); trivially diffable in git; dependency-free Rust parser fits the WASM target and headless unit testing (NFR1/NFR5); byte-stable canonical output is easy to guarantee and test (Q1); version header leaves room for collections (Obj. 3) and connectors without breaking old files.
- **Negative / costs:** we own the grammar — it must be specified precisely and parser/serializer kept in lockstep; label escaping rules are needed (quotes, `#`, newlines inside text); no off-the-shelf tooling or editor integrations at first.
- **Follow-ups:** ~~create concept doc `diagram-file-format`~~ done in S-003 — the full grammar is normative in [diagram-file-format](../arc42/12-concepts/diagram-file-format.md) (C6), with template/collection syntax in [shape-template](../arc42/12-concepts/shape-template.md) (C7) and [collection](../arc42/12-concepts/collection.md) (C8). All former open points (statement syntax + shape IDs, label escaping, connector statements, template extension mechanism) are settled there. ~~Implement `ad-format`~~ done in S-004 (`crates/ad-format`: parser + canonical serializer, lossless round-trip proven by proptest). Remaining: collections loading (S-005); glossary entry in S-021.

## Version history

- **v3 (2026-10-23):** decision implemented — `ad-format` parser + canonical serializer landed in S-004 (`crates/ad-format`, 86% line coverage, golden + property round-trip tests); status flipped to `accepted`. Collections loading (S-005) remains a follow-up.
- **v2 (2026-10-23):** grammar settled by S-003 — concept docs C6/C7/C8 created and linked; the illustrative sketch is marked superseded. Decision unchanged; status stays `proposed` until S-004/S-005 implement it.
- **v1 (2026-10-18):** initial decision.
