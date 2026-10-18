---
title: 006 - svgbob-compatible-ascii-alphabet
description: the canonical resolved grid uses svgbob's ASCII input alphabet, so text output is directly svgbob-parseable and no transliteration adapter is needed
scope: rendering
status: proposed
updated: 2026-10-18
supersedes: 001
---

# ADR 006: svgbob-compatible ASCII as the canonical diagram alphabet

## Context

[ADR 005](005-use-svgbob-as-vector-renderer.md) delegates grid→vector interpretation to the `svgbob` crate. Until now two alphabets coexisted: [ADR 001](001-unicode-glyph-matching-renderer.md) (accepted) fixed text output to **Unicode box-drawing characters** (`─ │ ┌ …`), while svgbob accepts only its own **ASCII input alphabet** (`+ - | / \`, rounded corners `. , ' `` ` `, dashes `- - - ~ _ : !`, arrows `< > ^ v V`, ports/circles `* o O`, double-quote escape for literal text). The plan was to bridge them with a deterministic Unicode→ASCII transliteration adapter.

That bridging has real costs:

- A second code path (the adapter) that must stay in sync with both our glyph table and the svgbob spec, with its own golden-file tests.
- The text output is **not machine-readable** by any existing svgbob tool (kroki, zola, mdbook, svgbob-editor): users can paste our diagrams as *art*, but cannot round-trip or edit them in the ecosystem that defines our rendering model.
- Product direction (§1: "Visio-like visual editor for SVG-compatible diagrams", "rendering similar to svgbob") pulls toward a single documented alphabet — svgbob's.

The user explicitly requested (2026-10-18): if we use svgbob, the ASCII output must be svgbob-compatible.

## Decision

**The canonical resolved grid uses svgbob's input alphabet.** Specifically:

1. **Glyph resolution (C1) targets svgbob characters.** Each occupied cell resolves to a character from svgbob's documented set — lines `- | / \`, sharp corners/junctions `+`, rounded corners `. , ' `` ` `, dashed lines (`- - -`, `~`, `_`, `:`, `!`), arrows `< > ^ v V`, ports/circles `* o O`.
2. **Text output (FR2) is the resolved grid itself** — pure ASCII, monospace, pasteable anywhere *and* directly parseable by svgbob and its ecosystem tools.
3. **SVG export (FR5) = `svgbob::to_svg(text)` on exactly those bytes.** The transliteration adapter from ADR 005 is dropped; there is nothing left to translate. Text and SVG are one representation, two outputs.
4. **Labels** containing characters svgbob would interpret as drawing are emitted with svgbob's double-quote escape (`"…"`) in the grid itself, so text output and SVG stay identical.

This supersedes ADR 001's Unicode-alphabet decision. The *mechanism* of glyph resolution — one character per cell, chosen from all edges/content passing through it — is retained; only the target alphabet changes.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Keep Unicode box-drawing text + transliteration adapter (ADR 005 as originally written) | Extra code and tests for a mapping no one else needs; text output stays opaque to the svgbob ecosystem; two alphabets to keep in sync forever |
| Fork/extend svgbob to accept Unicode box-drawing input | Upstream divergence for a feature we don't need once our own alphabet *is* svgbob's |

## Consequences

- **Positive:** single source of truth for both outputs — SVG is a pure function of the text output; no adapter to build, test, or maintain; text diagrams are directly editable in svgbob tooling (kroki, zola, mdbook, svgbob-editor); pure ASCII renders in every terminal and font (stronger "paste anywhere" than Unicode); the glyph-resolution table becomes the upstream svgbob spec — documented and stable.
- **Negative / costs:** plain-text look loses the visual density of Unicode box-drawing (mitigated: rounded corners + arrows are classic, readable ASCII art); labels with special characters need quote escaping in the text output; any future content outside svgbob's vocabulary degrades to literal text glyphs — the same feature ceiling ADR 005 already accepted.
- **Follow-ups:** define the exact per-connection-pattern glyph mapping (which patterns use sharp `+` vs rounded corners) and update concept C1 ([glyph resolution](../arc42/12-concepts/glyph-resolution.md)); verify label escaping against svgbob's parser behavior in golden tests; on acceptance, flip ADR 005 to accepted once the pipeline is implemented.
