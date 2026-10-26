---
title: 001 - glyph-resolution-alphabet
description: the renderer resolves each occupied grid cell to one character from a fixed alphabet; since v2 that alphabet is svgbob's ASCII input set
scope: rendering
status: accepted
version: 3
updated: 2026-10-26
---

# ADR 001: Glyph resolution onto a fixed character alphabet

## Context

The project name says "ASCII", but v1 of this decision made the output **Unicode**: box-drawing characters (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`, …). When shape edges meet or cross on the shared cell grid, one cell can belong to several edges — the renderer must decide which single character to display there.

Since v2, [ADR 005](005-use-svgbob-as-vector-renderer.md) delegates grid→vector interpretation to the `svgbob` crate, which accepts only its own **ASCII input alphabet** (`+ - | / \`, rounded corners `. , ' `` ` `, dashes `- - - ~ _ : !`, arrows `< > ^ v V`, ports/circles `* o O`, double-quote escape for literal text). Bridging the two alphabets with a transliteration adapter has real costs:

- A second code path (the adapter) that must stay in sync with both our glyph table and the svgbob spec, with its own golden-file tests.
- The text output is **not machine-readable** by any existing svgbob tool (kroki, zola, mdbook, svgbob-editor): users can paste our diagrams as *art*, but cannot round-trip or edit them in the ecosystem that defines our rendering model.
- Product direction (§1: "Visio-like visual editor for SVG-compatible diagrams", "rendering similar to svgbob") pulls toward a single documented alphabet — svgbob's.

The user explicitly requested (2026-10-18): if we use svgbob, the ASCII output must be svgbob-compatible.

## Decision

**The canonical resolved grid uses svgbob's input alphabet.** Specifically:

1. **Glyph resolution targets svgbob characters.** Each occupied cell resolves to a character from svgbob's documented set — lines `- | / \`, sharp corners/junctions `+`, rounded corners `. , ' `` ` `, dashed lines (`- - -`, `~`, `_`, `:`, `!`), arrows `< > ^ v V`, ports/circles `* o O`.
2. **Text output (FR2) is the resolved grid itself** — pure ASCII, monospace, pasteable anywhere *and* directly parseable by svgbob and its ecosystem tools.
3. **SVG export (FR5) = `svgbob::to_svg(text)` on exactly those bytes.** The transliteration adapter from ADR 005 is dropped; there is nothing left to translate. Text and SVG are one representation, two outputs.
4. **Labels** containing characters svgbob would interpret as drawing are emitted with svgbob's double-quote escape (`"…"`) in the grid itself, so text output and SVG stay identical.

The *mechanism* of glyph resolution is unchanged from v1: one character per cell, chosen from all edges/content passing through it — only the target alphabet changed (Unicode box-drawing → svgbob ASCII).

## Alternatives considered

**v1:**

| Alternative | Why rejected |
|-------------|--------------|
| Pure ASCII (`+ - \|`) | Less readable; Unicode output is desired despite the project name |
| Static per-shape glyphs (no junction resolution) | Corners/junctions break where shapes meet or touch |

**v2 (current):**

| Alternative | Why rejected |
|-------------|--------------|
| Keep Unicode box-drawing text + transliteration adapter (ADR 005 as originally written) | Extra code and tests for a mapping no one else needs; text output stays opaque to the svgbob ecosystem; two alphabets to keep in sync forever |
| Fork/extend svgbob to accept Unicode box-drawing input | Upstream divergence for a feature we don't need once our own alphabet *is* svgbob's |

## Consequences

- **Positive:** single source of truth for both outputs — SVG is a pure function of the text output; no adapter to build, test, or maintain; text diagrams are directly editable in svgbob tooling (kroki, zola, mdbook, svgbob-editor); clean joins between shapes via glyph resolution; pure ASCII renders in every terminal and font (stronger "paste anywhere" than Unicode); the glyph-resolution table becomes the upstream svgbob spec — documented and stable.
- **Negative / costs:** plain-text look loses the visual density of Unicode box-drawing (mitigated: rounded corners + arrows are classic, readable ASCII art); labels with special characters need quote escaping in the text output; any future content outside svgbob's vocabulary degrades to literal text glyphs — the same feature ceiling ADR 005 already accepted.
- **Follow-ups:** define the exact per-connection-pattern glyph mapping (which patterns use sharp `+` vs rounded corners) and update concept C1 ([glyph resolution](../arc42/12-concepts/glyph-resolution.md)); verify label escaping against svgbob's parser behavior in golden tests; on acceptance, flip ADR 005 to accepted once the pipeline is implemented.

## Version history

- **v3 (2026-10-26):** decision implemented — S-006 landed the resolve stage of `crates/render-pipeline`: per-cell rule table over all 16 connection sets emitting svgbob-alphabet glyphs, double-quote label escaping in the grid itself, and proptest-enforced alphabet purity (T3). Per-pattern mapping defined and recorded in concept C1 (`glyph-resolution.md`, incl. fallback rules F1–F5); status flipped to `accepted`. ADR 005 stays `proposed` until its own implementation (SVG emission via `svgbob::to_svg`) lands in S-008.
- **v2 (2026-10-18):** canonical alphabet switched from Unicode box-drawing to svgbob's ASCII input alphabet; transliteration adapter dropped. Originally recorded as ADR 006; merged into this record on 2026-10-18 because ADRs are now versioned in place instead of superseded (see AGENTS.md).
- **v1 (2026-10-17):** initial decision — output uses Unicode box-drawing characters; the renderer resolves each occupied cell to the matching glyph from all edges/content passing through it. Recorded and accepted 2026-10-17.
