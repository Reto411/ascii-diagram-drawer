---
title: diagram-file-format
description: the .ad diagram file grammar — versioned, line-based, one statement per line with explicit cell coordinates; read before implementing or changing ad-format (S-004)
scope: format
status: stable
updated: 2026-11-01
---

# Concept: Diagram File Format (`.ad`)

## Definition

The `.ad` file is the canonical, versioned, line-based plain-text serialization of a `Diagram` aggregate ([ADR 004](../../adr/004-diagram-file-format.md)): one statement per physical line, explicit cell-grid coordinates, no layout engine. It is the source of truth for persistence (FR4) and the only format whose round-trip must be lossless (QS7).

## Why it exists

- FR4: save/load with reload → identical model.
- NFR2/Q3: `cat file.ad` shows the diagram's structure without any renderer.
- NFR3/Q1: deterministic — the same model always serializes to byte-identical bytes.
- ADR 004 settled *that* it is a custom line-based format; this document settles *exactly which* grammar (the open points ADR 004 deferred).

## Format family

One versioning scheme covers all three file types:

| File | Extension | Header | Content | Spec |
|------|-----------|--------|---------|------|
| Diagram | `.ad` | `ad <major>.<minor>` | diagram statements (this doc) | here |
| Collection | `.adc` | `adc <major>.<minor>` | shape template declarations | [collection](collection.md) |
| Template | — (inline in `.adc`) | — | grid + anchors | [shape-template](shape-template.md) |

## Diagram file grammar (v1 family — `ad 1.0` core + `ad 1.1` extensions)

```ebnf
file         = header, statement* ; statements in stored object order
header       = "ad", int, ".", int ; must be the FIRST statement of the file
statement    = scale-stmt / colors-stmt / shape-stmt / text-stmt / connect-stmt / unknown-stmt
scale-stmt   = "scale", int        ; at most once; must precede every object statement
colors-stmt  = "colors", ( color-role, string )+ ; at most once; must precede every object statement (1.1, CR-009)
color-role   = "background" / "fill" / "lines" ; each at most once per statement
shape-stmt   = "shape", type-id, string, "as", id, "at", int, int, "w", int, "h", int, region-clause*   ; region clauses: 1.1 (FR19)
region-clause= "region", name, string
text-stmt    = "text", "as", id, string, "at", int, int
connect-stmt = "connect", "as", id, "from", endpoint, "to", endpoint, [ "dashed" ], [ "tip", tip-style, tip-style ], [ "via", via-cell ( "," via-cell )* ]
endpoint     = anchor-ref / int, int ; attached '<id>.<anchor>' or free at a bare cell (1.1, CR-010)
tip-style    = "none" / "arrow" / "rhombus-open" / "rhombus-filled"
via-cell     = int, int
anchor-ref   = id, ".", anchor     ; split at the LAST dot (ids/anchors may contain dots); both parts non-empty
type-id      = token / collection, "/", name ; bare = built-in shape; prefixed = user template
id, anchor, token, collection, name = single token : 1+ chars, no whitespace, no leading/trailing space
string       = '"', ( char / escape )*, '"'
escape       = "\"\"" / "\\\\" / "\\n" ; the ONLY valid escapes in v1
int          = "0" / digit1to9, digit* ; canonical decimal — no leading zeros; fits u32
```

### Statement semantics

- **`shape <type-id> "<label>" as <id> at <x> <y> w <width> <height> [region <name> "<content>"]*`** — a `ShapeInstance`. `type-id` is the only reference into the Shape Catalog context (built-in or `collection/name`). Label may be empty (`""`). Zero or more **region clauses** (1.1, FR19) carry per-region text content: names are single tokens, unique per statement (a duplicate is a parse error); content is a quoted string with the standard escapes and may be empty (an empty clause is still emitted — it pins "no content"). Region names are *not* validated against the catalog (same rule as `type-id`): unknown names round-trip but render nothing. Canonical serialization emits clauses sorted by region name; shapes without regions serialize exactly as before.
- **`text`** — a `TextInstance`: `text as <id> "<content>" at <x> <y>`. Content must be non-empty (model invariant). No size: text occupies exactly the cells its (possibly multi-line) content needs.
- **`connect as <id> from <endpoint> to <endpoint> [dashed] [tip <src> <tgt>] [via <x> <y>, …]`** — a `ConnectorInstance`. Style omitted = solid (the model default); `dashed` = dashed. `tip <src> <tgt>` sets the per-end tip styles (CR-008): omitted = legacy default (source none, target arrow). `via` is the ordered list of manual waypoints in model-space cells (CR-007). Each endpoint is either `<id>.<anchor>` (attached — must reference a *shape* object; ids are file-local, so **forward references** to shapes declared later in the file are legal — references resolve only after the whole document has been read) or two bare integers `<x> <y>` (free at a model-space cell, CR-010); the token shape disambiguates (a dot ⇒ attached), and both-free endpoints may not share a cell.
- **`scale <n>`** — whole-diagram scale (FR8), `n ≥ 1`; omitted means 1. The canonical serializer always emits it.
- **`colors [<role> "#rrggbb"]+`** — diagram-level presentation colors (CR-009, FR15): bare role words (`background`, `fill`, `lines`) in any order, each at most once, with a quoted lowercase-compatible hex value; at least one pair required. Like `scale`, it must precede every object statement and appear at most once per file. Omitted = all roles unset (the monochrome default).

### Canonical serialization (Q1/NFR3)

The writer is deterministic; "canonical form" means:

1. Header first, then the `scale` statement (always emitted, even `scale 1`), then the `colors` statement (only when any role is set — rule 4), then all objects **in stored order** — shapes, text, and connectors interleaved exactly as in the model's object list (file order = z-order).
2. Exactly one space between tokens; no leading indentation; no trailing whitespace; LF line endings; no blank lines; no comments (comments/blank lines are decoration, never model data).
3. Integers in canonical decimal (no leading zeros); style keyword `dashed` emitted only for the non-default style; the `tip` clause only when the pair differs from the defaults; `via` only when non-empty.
4. The `colors` statement is emitted only when at least one role is set, immediately after `scale`, with roles in canonical order (`background`, `fill`, `lines`) and lowercase hex.
5. Property: `serialize(parse(serialize(d))) == serialize(d)` for all models `d`, and `parse` of a canonical file is byte-stable under re-serialization (QS7).

### Parsing rules (lenient in, strict out)

- Whitespace between tokens: any run of spaces/tabs; leading indentation and trailing whitespace on statement lines are stripped (tokenization leniency for *known* statements — for `unknown-stmt` the verbatim-preservation rule in [Versioning](#versioning-unknown-field-tolerance) takes precedence: they are re-emitted exactly as read, whitespace included).
- A line whose first token is **not** a known keyword (`scale`/`shape`/`text`/`connect`) is an `unknown-stmt` — see versioning below.
- A line whose first token **is** a known keyword but fails to parse is a hard error in every version (corruption, not forward-compatibility).
- Header version tokenization: the canonical header is `ad <major>.<minor>`; whitespace around the dot (`ad 1 . 0`) is accepted as input leniency — the serializer never emits it.
- Input CRLF is tolerated: `\r` is whitespace to the tokenizer (never word content), and any run of trailing `\r` before a line break or at end-of-file is stripped as line-break decoration from preserved lines (one = classic CRLF; longer runs normalize identically, since re-emitting them could not round-trip). Output is LF only. An interior `\r` inside a preserved line is content and round-trips verbatim.
- Comments: `#` outside a string starts a comment running to end of line. Whole-line and inline comments are both valid.

### Versioning + unknown-field tolerance

Header `ad <major>.<minor>`; v1 ships as `ad 1.0`. A reader of version `M.m` handling a file of version `f.M.f.m`:

- `f.M ≠ M` → **reject the whole file** with an error (no silent data loss).
- `f.M = M`, `f.m > m` → forward-compatible: unknown statements are **skipped and preserved verbatim**; all known statements must still parse.
- `f.M = M`, `f.m ≤ m` → every statement must be a known, well-formed one (an older writer cannot emit unknowns).

Preserved unknown statements are re-emitted byte-for-byte at their original position between known statements on save — QS7 holds across minor-version upgrades too.

### Decided extensions (minor version 1.1, accepted 2026-10-31)

The CR batch (CR-001…CR-018) extends the family to `ad 1.1` / `adc 1.1`. The existing versioning + skip-and-preserve rules apply unchanged; each extension below lands in its story's commit, where the normative grammar section of this doc (and C7/C8 for `.adc`) is updated with the exact statement syntax. Writers never emit statements they don't understand, so intermediate builds stay legal.

| Extension | Encoding sketch | Story |
|-----------|-----------------|-------|
| Connector manual bends (CR-007) | `connect` gains an optional ordered waypoint list (`via <x> <y>, …`) in model-space cells | S-028 |
| Per-end connector tips (CR-008) | `connect` gains optional per-end tip keywords (none / arrow / rhombus open / rhombus filled); absent = old default (source none, target arrow) | S-029 |
| Free connector endpoints (CR-010) | a `connect` endpoint may be a bare cell position (`at <x> <y>`) instead of `<id>.<anchor>`; attach/detach is GUI-only | S-031 |
| Diagram colors (CR-009) | diagram-level statements for background / shape-fill / line-text color as `#RRGGBB`; absent = current monochrome rendering | S-030 |
| Shape-owned text regions (CR-016, ADR 004 v4) | `region` declarations inside `.adc` template blocks (name + cell rect); `shape` statements gain per-region quoted content; the existing single label is preserved as-is | S-037 |
| Template resize rules (CR-003, ADR 007 v6) | `resize` statements inside `.adc` template blocks — grammar normative in [resize-rules](resize-rules.md) (C10), exact syntax settled by S-024 | S-024 |

Round-trip (QS7) and byte-stability extend to every new field: each model field gets exactly one representation, and `serialize(parse(serialize(d))) == serialize(d)` keeps holding proptest-enforced.

## Field mapping (QS7: exactly one representation per model field)

| `diagram-model` field | `.ad` representation |
|---|---|
| `Diagram.scale` | `scale` statement; absence = 1 |
| object order / z-order | physical order of object statements |
| `ShapeInstance.id` / `TextInstance.id` / `ConnectorInstance.id` | `as <id>` (unique across the file) |
| `ShapeInstance.type_id` | `<type-id>` token (bare or `collection/name`) |
| `ShapeInstance.label` | quoted `<string>` after the type-id |
| `ShapeInstance.pos` / `TextInstance.pos` | `at <x> <y>` |
| `ShapeInstance.size` | `w <width> h <height>` (both ≥ 1) |
| `TextInstance.text` | quoted `<string>` (non-empty) |
| `ConnectorInstance.from` / `.to` | `from <endpoint>` / `to <endpoint>` — `<id>.<anchor>` (attached) or `<x> <y>` (free, CR-010) |
| `ConnectorInstance.style` | absent = solid; `dashed` = dashed |
| `Diagram.colors` (`background`/`fill`/`lines`) | `colors` statement — one quoted `#rrggbb` per set role; absent = all unset (CR-009) |

No model field is omitted or duplicated; nothing in the file maps to no model field (except preserved unknown statements, which are opaque pass-throughs).

## Edge cases (settled rules)

1. **Escapes.** Only `\"`, `\\`, `\n` exist. Any other backslash sequence is a parse error. A raw line break can never occur inside a string — every statement fits on one physical line; multi-line text uses repeated `\n`.
2. **Empty labels.** Shapes: allowed (`shape box "" as u1 ...`). Text objects: rejected at load (model invariant `EmptyText`).
3. **Overlapping objects.** Allowed with no constraint. File order = z-order: later statements overwrite earlier ones on shared cells (see [cell grid](cell-grid.md)).
4. **Connectors — no default anchor in v1.** Every endpoint names an explicit anchor (`obj.anchor`), even when the shape type declares a single one. There is no `from u1` shorthand; the GUI keeps this rule too — connecting always targets the specific anchor the user drops onto (S-013 drag-to-anchor), and automatic "pick a sensible anchor" selection is not a v1 feature, format or GUI.
5. **Referential integrity at load.** Duplicate ids, connector endpoints naming missing objects or non-shapes, zero sizes, scale 0, multi-line statements: all rejected with line/column errors — the parser assembles through the `Diagram` aggregate API, so every model invariant applies (see [diagram aggregate](diagram-aggregate.md)).
6. **Integers.** Canonical decimal, no leading zeros, u32 range; overflow or `+`/`-` signs are parse errors (negative coordinates are unrepresentable by construction).
7. **Comments and blank lines** carry no information: the parser discards them, the serializer never emits them. Hand-edited files gain/lose none of the model by reformatting.

## Relations

- **Diagram aggregate** — this format is its serialization; field names mirror the API (link: `doc/arc42/12-concepts/diagram-aggregate.md`)
- **Cell grid** — coordinates are cell units on the unbounded grid; scale is a whole-diagram property (link: `doc/arc42/12-concepts/cell-grid.md`)
- **Shape template / Collection** — `collection/name` type-ids resolve into `.adc` files (links: `doc/arc42/12-concepts/shape-template.md`, `doc/arc42/12-concepts/collection.md`)
- **Glyph resolution** — the *rendered* output uses svgbob's alphabet (ADR 001); the `.ad` file is a different, structural representation — never the rendered art (ADR 004).

## Implementation

Implemented in `crates/ad-format` (S-004, ADR 004 accepted):

- **Parser** — `src/parser.rs`: `parse_document(&str) -> Result<Document, ParseError>`; line-based tokenizer with 1-based line/column on every error; assembles through the `Diagram` aggregate API so all model invariants apply; versioning + unknown-statement preservation per the rules above. `ParseError` (`src/error.rs`) carries line, column and message.
- **Canonical serializer** — `src/serializer.rs`: `serialize(&Document) -> String`; emits the canonical form (header, `scale`, objects in stored order, one space between tokens, LF endings); re-saves byte-stable; preserved unknown statements are re-emitted verbatim at their original position.
- **Tests** — unit tests (error positions, versioning, edge cases) in the crate, golden files in `tests/golden/*.ad`, and a proptest lossless round-trip (`tests/round_trip.rs`, QS7).

Still to come: `.adc` collection loading + type-id resolution against the catalog is S-005 (`shape-registry`); this spec's `.ad` part is complete.
