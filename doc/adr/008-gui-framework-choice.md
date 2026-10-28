a---
title: 008 - gui-framework-choice
description: which GUI framework targets native desktop AND web/WASM from one Rust codebase — decided egui/eframe via weighted decision matrix (status proposed); read before any GUI code
scope: GUI / tech stack
status: proposed
updated: 2026-10-18
version: 2
---

# ADR 008: GUI framework choice

## Context

The GUI must run on **Windows, macOS, Linux (native desktop) and in the browser (WASM) from one codebase** ([NFR1](../arc42/14-important-requirements.md), Q2, [§5.3](../arc42/05-constraints.md)). The app is a **canvas editor**: grid canvas with place/drag/resize of objects, live re-render while dragging (≤16 ms/frame target), pan/zoom, and a template sidebar — i.e. heavy custom 2D drawing plus standard widgets.

Fixed by earlier ADRs: the platform-independent core (model, `.ad` format, render pipeline, svgbob SVG) is already framework-agnostic Rust; whatever GUI we pick only has to (a) draw the canvas, (b) translate user gestures into model mutations, (c) call the core — natively on desktop and via WASM in the browser.

**Decision status: decided 2026-10-18** (joint evaluation; weights and scores agreed as drafted). Status stays `proposed` until the first GUI milestone is implemented.

## Candidates & Evaluation

### Candidate evidence (researched 2026-10-18)

| Candidate | Desktop (Win/macOS/Linux) | Web/WASM | UI model | Evidence / notes |
|---|---|---|---|---|
| **egui / eframe** | ✅ official | ✅ official, first-class ("runs on both web and native"; demo at egui.rs uses wasm+WebGL; "All code in `egui` is Wasm-friendly") | Immediate mode; `egui::Painter` custom drawing | README: eframe targets Web, Linux, Mac, Windows, Android. Caveat: project states interfaces are still in flux (breaking changes between releases) |
| **Slint** | ✅ primary target | ✅ official — Rust only (winit backend + FemtoVG software renderer) | Retained mode; declarative `.slint` markup; built-in vector scene graph (paths/shapes) | slint.dev: "build native UIs for embedded, desktop, and web from a single codebase". Commercially backed (Slint AG); stable releases |
| **Dioxus** | ✅ official | ✅ official — core design goal ("Build for web, desktop, and mobile … with a single codebase") | Retained mode; React-like components + signals | Fullstack CLI, hot reload. Canvas/2D-drawing story on the *desktop* renderer needs verification (DOM-style model) |
| **iced** | ✅ official | ⚠️ claimed in README ("Windows, macOS, Linux, and the Web") — maturity unverified | Retained mode; Elm-like reactive; wgpu / tiny-skia renderers | Type-safe, batteries included. Web target is new — needs a spike to verify quality before scoring counts |
| **Tauri + SvelteKit** (hybrid) | ✅ via system webview | ✅ SvelteKit static site | Web app (HTML5 canvas/SVG) wrapped in a native shell on desktop | Matches the repo's `.gitignore` hints (SvelteKit/Vite). Most mature 2D editing tooling (browser canvas APIs), but UI code is TypeScript + Rust core via WASM/JS boundary — two languages, and "native" = webview |

Considered and dropped: **Flutter** (mature desktop+web, but Dart UI contradicts the all-Rust workspace convention); **druid** (low activity); **GPUI/Zed** (young, built for one editor, not a general toolkit).

### Weighted decision matrix (agreed in joint review, 2026-10-18)

Gates must pass; scores 1–5 (5 = best fit); weights sum to 12.

| Criterion (weight) | Why it matters | egui/eframe | Slint | Dioxus | iced | Tauri + SvelteKit |
|---|---|---|---|---|---|---|
| **G1** one codebase → desktop + web *(gate)* | NFR1/Q2 — non-negotiable | ✅ | ✅ | ✅ | ⚠️ unverified | ✅ (webview) |
| **G2** permissive OSS license *(gate)* | §5.1 | ✅ MIT/Apache | ✅ MIT | ✅ Apache/MIT | ✅ MIT | ✅ (Tauri Apache; SvelteKit MIT) |
| Canvas editing UX fit (3) | Custom grid drawing, hit-testing, live drag re-render, pan/zoom | **5** — Painter API + immediate-mode redraw loop is the natural canvas-editor model | **4** — built-in vector scene graph; grid-level hit-testing needs custom work | **3?** — DOM-style model; desktop 2D-drawing story unverified | **3** — custom-widget drawing possible, less proven | **5** — HTML5 canvas/SVG: most mature browser 2D tooling |
| Rust core integration (3) | GUI calls the same core natively (desktop) and via WASM (web); one code path | **4** — all-Rust; trivial native calls, same code compiles to wasm | **4** — all-Rust, native + wasm | **4** — all-Rust, native + wasm | **4** — all-Rust, native + wasm | **2** — core is WASM behind a JS boundary on *both* targets; per-interaction FFI overhead & two languages |
| Maturity / stability (2) | We are a small team; churn costs real time | **3** — active but "interfaces still in flux" | **4** — stable releases, commercial backing | **3** — active, younger API surface | **3** — active; web target new | **5** — Tauri v2 + SvelteKit both battle-tested |
| Ecosystem for our needs (2) | File dialogs, clipboard, text layout, export plumbing | **3** — eframe native APIs cover basics; text layout basic (fine for labels) | **3** — dialogs/clipboard supported | **3** — web-sys escape hatch + desktop plugins | **3** — batteries included | **4** — browser APIs are the reference standard here |
| Performance & WASM size (1) | ≤16 ms/frame; <2 MB compressed budget (§5.3, proposed) | **4** — WebGL/wgpu, moderate bundle | **3** — FemtoVG = CPU software rendering in browser (fine at our scale) | **3** — TBD per target | **3** — wgpu native; web renderer TBD | **3** — two bundles (wasm + JS); browser perf excellent |
| Team fit / maintainability (1) | Long-term: Rust-only UI code is a plus for this repo | **5** — pure Rust | **5** — Rust (+`.slint` markup) | **5** — pure Rust | **5** — pure Rust | **2** — TS/Svelte + Rust to maintain in parallel |
| **Weighted total (max 50)** | | **48** | **46** | **41** | **41** ⚠️ | **44** |

> *`?`/⚠️ cells: iced web maturity and Dioxus desktop canvas story remain unverified — moot for the outcome, since both candidates trail the top three by ≥3 points under the agreed scores.*

## Decision

**egui / eframe** (Rust, immediate mode) is the GUI framework:

- One Rust codebase compiles to native desktop (Windows/macOS/Linux via `egui-winit` + wgpu/glow) **and** browser wasm (WebGL) — satisfies NFR1/Q2 with an official, demoed web target.
- The immediate-mode redraw loop *is* our canvas-editor model: every gesture invalidates → grid + shapes re-rendered; no retained scene-graph synchronization to build. `egui::Painter` provides custom 2D drawing; interaction/hit-testing is built in.
- Core integration is all-Rust on **both** targets — the GUI calls the same platform-independent core directly (desktop) and inside wasm (web). No JS boundary, no second UI language.

**Fallback:** Slint (46) if eframe's API churn proves painful during the first GUI milestone — revisit by bumping this ADR's `version`, not a new ADR.

Not selected: Tauri + SvelteKit (44 — most mature canvas tooling, but TS+Rust two-language UI and a wasm/JS core boundary on *both* targets); Dioxus (41 — desktop 2D-drawing story unverified); iced (41⚠️ — web support unverified).

Remaining verification happens *inside* the first GUI milestone, not before it: workspace scaffold + spike (grid canvas with drag + live re-render, built for desktop **and** browser), plus per-target checks of file dialogs/clipboard.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Flutter (Dart) | Excellent desktop+web canvas support, but UI in Dart contradicts the all-Rust workspace convention (AGENTS.md); FFI core integration adds a second boundary |
| druid | Low maintenance activity — risky long-term bet |
| GPUI (Zed) | Built for Zed's editor; young API; not positioned as a general application toolkit |
| Two separate GUIs (native + web, no shared UI code) | Violates NFR1/Q2 ("one codebase") and doubles UI maintenance — the exact cost this ADR exists to avoid |

## Consequences

- **+** Single language end-to-end; first-class web target; `Painter` API purpose-built for custom drawing; fast iteration on a canvas app.
- **−** egui/eframe APIs are explicitly in flux (breaking changes between releases). Mitigation: pin versions, keep the GUI layer thin so churn never reaches the platform-independent core; revisit via ADR version bump if it hurts.
- **−** Immediate mode re-renders every frame. Fine at our scale boundary (§5.2: ≤500 objects) because the pure core pipeline bounds per-frame cost deterministically — but canvas draw calls must stay bounded (no per-cell allocations in hot paths).
- **−** No DOM conveniences: file dialogs/clipboard come from eframe native APIs or small crates (`rfd`, `arboard`) — each must be verified on all four targets during the first milestone.
- The SvelteKit/Vite entries in `.gitignore` are obsolete hints (no separate web frontend); cleanup optional.

## Version history

- **v2 (2026-10-18):** joint evaluation completed — weights and scores agreed as drafted; **egui/eframe selected** (48/50), Slint recorded as fallback (46). Verification of trailing candidates deprioritized. Decision pending implementation (status `proposed`).
- **v1 (2026-10-18):** opened for joint evaluation — candidate evidence gathered, weighted matrix drafted (provisional). No decision yet.
