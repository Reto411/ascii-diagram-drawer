# ascii-diagram-drawer

Draw ASCII diagrams **visually**: place, move, and resize shapes on a grid canvas, and get clean monospace text you can paste anywhere. No hand-typed box-drawing characters — and no auto-layout engine quietly rearranging your diagram into spaghetti.

```text
+-------+     +-------+
|       |     |       |
|   A   +---->+   B   |
|       |     |       |
+-------+     +-------+
```

*Real output of the render pipeline — what you see on the canvas is exactly the text you get.*

## Why

- **You control the layout.** Shapes snap to a grid and are placed by hand. Rendering is a pure, deterministic function of the model — the same diagram renders byte-identically everywhere, on every platform.
- **Plain text first.** Output is monospace text with Unicode box-drawing characters: paste it into terminals, READMEs, chat, commit messages, docs.
- **SVG and PNG too.** Vector export via [svgbob](https://ivanceras.github.io/svgbob/); raster as an optional backend.
- **Custom shapes & collections.** Beyond the built-in box/ellipse: define your own diagram objects from text art and reuse them across diagrams.

## Status

Early development, driven by user stories — see the [board](doc/stories/BOARD.md):

- ✅ done: diagram model, plain-text `.ad` format (parse/serialize), shape catalog + custom templates/collections, deterministic render pipeline (model → text grid) — unit-tested incl. golden files
- 🔨 in progress: anchor-bound connectors
- ⏳ next: SVG/PNG export, then the canvas GUI (egui/eframe — desktop **and** browser via wasm), file I/O, scale control

## Local LLMs only

This project doubles as an experiment in autonomous development with **local** LLM models: everything so far was implemented by local LLM agents (a Qwen3.8-27B GGUF served on local hardware via [Unsloth](https://github.com/unslothai/unsloth)) — **no cloud LLM APIs were used at any point**. The agents run in an isolated devcontainer ([.devcontainer/README.md](.devcontainer/README.md)) with an allow-listed egress firewall, so they cannot reach external AI services.

## How it works

A layered Rust workspace: headless core (model → format → shapes → render → export), an application service seam, and a thin egui presentation layer on top. All rendering logic is platform-independent and unit-testable without a GUI; the GUI draws the resolved grid and translates gestures into model mutations.

| Crate | Role |
|-------|------|
| `diagram-model` | core domain: Diagram aggregate, cell geometry, anchors, invariants |
| `ad-format` | `.ad` file format — parse/serialize (ADR 004) |
| `shape-registry` | shape types, glyph patterns, anchor sets, collections (ADR 007) |
| `render-pipeline` | place → resolve → text grid, glyph resolution (ADR 001) |
| `exporters` | text / SVG / PNG emission from the grid (ADR 005) |
| `ascii-import` | importing foreign ASCII diagrams — anti-corruption layer (v1+) |
| `ui-api` / `core-services` | service seam between GUI and core (ADRs 009, 012) |
| `gui-app` | eframe/egui app — native desktop + browser wasm (ADR 008) |

## Getting started

```sh
cargo build --workspace
cargo test --workspace
```

Rust is pinned in [rust-toolchain.toml](rust-toolchain.toml). The architecture, decisions, and progress all live in the repo:

- [arc42 architecture docs](doc/arc42/overview.md) — objectives (§1.1), building blocks, quality model
- [ADRs](doc/adr/) — every design decision with alternatives considered
- [story board](doc/stories/BOARD.md) — what is done and what comes next

**Contributing:** work is story-driven — pick a ready story from the board (or propose one) and implement it against its acceptance criteria. Agent-assisted development runs in an isolated devcontainer: [.devcontainer/README.md](.devcontainer/README.md).

## License

[Apache-2.0](LICENSE)
