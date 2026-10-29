---
title: handoff-s019
description: session handoff — S-018 done and committed; what the next session needs to start S-019 (wasm milestone), incl. wasm tooling recon
scope: delivery / gui-app / devcontainer
status: stable
updated: 2026-10-29
---

# Handoff: S-018 done → S-019 next

**State:** S-018 committed (`dfebc60` "S-018: free text elements (FR10) …"). Board, story file (all 4 ACs verified, tier-1/2 visual ⚠️ deferred to image rebuild like S-012…S-017), overview tracker, C4 concept doc all updated in the same commit. Tree clean; In progress empty. Workspace: fmt/clippy(0)/test green (357 passed); tarpaulin 82.89% ≥ 80%. All v1 GUI stories except S-019 are done — **S-019 is the only remaining feature story** (S-020/S-021 depend on it).

## Read before implementing (in order)

1. `doc/stories/S-019-wasm-web-milestone.md` — 5 ACs: bundle build, core tests on wasm32, no feature reduction, bundle size vs P3 (2 MB proposed), gates + tracker.
2. ADR 008 (accepted v3) — the milestone matrix this closes; `doc/arc42/15-risks.md` P3 budget line (stays "awaiting confirmation").
3. `.agents/notes/s012-eframe-gotchas.md` — eframe 0.37 API notes, incl. the wasm entry point (`WebRunner`; current `main()` is a native-only stub that compiles to nothing on wasm).

## Wasm tooling recon (done in this session — saves re-discovery)

- **Available:** `node` + `npm` binaries exist at /usr/bin (needed by `wasm-bindgen-test-runner`). Rust toolchain has the `wasm32-unknown-unknown` target installed (S-015 check used it).
- **Missing:** `trunk`, `wasm-pack`, `wasmtime` — none installed. No prebuilt binaries in `/var/cache/apt`.
- **Network (ADR 014 egress allow-list):** crates.io works through the proxy (`static.crates.io` reachable; cargo fetches fine) → `cargo install trunk` is viable but a long source compile (~10+ min). The **npm registry is blocked** (403 security policy) — no npm installs, no wasm-pack via npm. GitHub releases untested; assume blocked.
- **Implication for AC1:** plan on `trunk` via `cargo install trunk` (or a hand-rolled build: `cargo build --release --target wasm32-unknown-unknown` + `wasm-bindgen` CLI — the CLI binary ships as a dev-dep build artifact, no npm needed). eframe 0.37 web entry = `eframe::WebRunner` per its docs; an `index.html` + `Trunk.toml` will be new files.
- **Implication for AC2:** `wasm-bindgen-test` as dev-dependency in the five headless crates (fetches via cargo, fine); run with `wasm-bindgen-test-runner --node` — verify the runner binary's node compatibility first (it needs a recent-enough node; check `node --version`).
- Version pinning: match `wasm-bindgen-test` to the `wasm-bindgen` version eframe 0.37 pulls in (check `cargo tree`) to avoid duplicate-wasm-bindgen link errors.

## Gotchas from S-018 that touch S-019 scope

- gui-app's wasm build currently exits 0 with ~59 dead-code warnings (the whole app struct tree is unused until the `WebRunner` entry lands) — expect to silence most of them by actually wiring the entry.
- Native-only paths that need cfg-gating per AC3: file open/save + `.txt`/SVG/PNG export sinks (`fileio.rs`, `exportio.rs`), clipboard copy, and `PngWorker::for_target()` already picks the inline executor on wasm (ADR 012 — done in S-015/S-016).
- The winit X11 lib gap (tier-1/2 visual deferral) is a *container instance* issue; the Dockerfile already installs `libxcursor1 libxi6 libxkbcommon0` — if the image is rebuilt before S-019, the deferred smoke/golden checklists for S-012…S-018 become runnable (they're written into each story's Verification).
