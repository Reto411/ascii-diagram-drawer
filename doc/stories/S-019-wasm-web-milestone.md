---
title: S-019-wasm-web-milestone
description: full web target — wasm bundle of gui-app builds + core test suites run on wasm32 (QS4); bundle size measured against P3
scope: workspace/gui-app
status: done
updated: 2026-11-02
---

# S-019: web/wasm milestone — build + tests on wasm

## Why

NFR1/QS2: one codebase, native + browser; the *same* test suite must pass on both targets (QS4). P3 (bundle < 2 MB) is a proposed budget — measure and report here. This closes the "browser" half of ADR 008's milestone matrix.

## Dependencies

- S-012…S-018 — all GUI functionality present (web build should not be feature-reduced; cfg-gated fallbacks only where a platform lacks an API)
- S-011 — wasm inline PNG path compiled in

## Acceptance criteria

1. **AC1 — Web bundle builds.** A documented command produces a servable web bundle of `gui-app` for wasm32-unknown-unknown (tooling choice — eframe web feature + wasm-bindgen/wasm-pack or trunk — recorded in Implementation notes); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0. Check: build command runs to completion; artifact exists.
2. **AC2 — Core tests on wasm.** The headless crates' test suites (`diagram-model`, `ad-format`, `shape-registry`, `render-pipeline`, `exporters`) run on wasm32 via a documented harness (e.g. wasm-bindgen-test) and pass; goldens identical to native runs. Check: harness command + pass output recorded in Verification.
3. **AC3 — No feature reduction.** The web app exposes the same features as native; platform-specific fallbacks (file IO, clipboard, PNG inline) are cfg-gated and listed in Implementation notes with a pointer to §10 deployment view. Check: code review checklist in Verification.
4. **AC4 — Bundle size measured.** Final JS+wat/wasm size recorded against the 2 MB proposed budget (P3); if over, note it as an open risk in `doc/arc42/15-risks.md` (budget stays "awaiting confirmation" — not a gate). Check: number recorded.
5. **AC5 — Gates green.** All standard gates still pass on the host target; overview tracker updated (Q2/NFR1 verified at build+test level; manual browser smoke checklist written for a human).

## Implementation notes

**Tooling (AC1):** [Trunk 0.22](https://trunkrs.dev/) (baked into the devcontainer image — ADR 014 egress allow-list forbids runtime installs) + eframe 0.37's `WebRunner` + wasm-bindgen. No wasm-pack: Trunk drives cargo for `wasm32-unknown-unknown`, runs wasm-bindgen, and assembles the static bundle.

**Crate split:** `gui-app` is now lib + bin. The whole app moved from `main.rs` into `lib.rs` (modules unchanged); `crate-type = ["cdylib", "rlib"]` — `cdylib` is Trunk's wasm entry, `rlib` keeps the native binary and all existing tests linking. Two entry points over one codebase:

- native: `main.rs` → `gui_app::run_native()` (eframe `run_native`, undecorated Material window)
- web: `src/web.rs` — `#[wasm_bindgen] pub fn WebHandle::new() -> Result<WebHandle, JsValue>` + `start(canvas)`, wrapping eframe's `WebRunner` exactly per the eframe 0.37 docs; `src/webio.rs` holds the DOM escape hatches (file picker, download anchor, `localStorage`)

**Build command:** `cd gui-app && trunk build --release` → static bundle in `gui-app/dist/` (`index.html` + JS glue + `.wasm`), servable from any static file host (needs http(s), not `file://` — wasm fetch + ES-module CORS). `Trunk.toml` pins the index; `gui-app/index.html` carries the `<link data-trunk rel="rust" data-target-name="gui_app">` tag (`data-target-name` disambiguates the lib from the same-named bin) and `data-wasm-opt="0"` (see AC4 — sandbox egress blocks the binaryen download; removing the attribute re-enables optimization where github.com is reachable).

**Test harness (AC2):** `scripts/wasm-core-tests.mjs` (Node ≥ 18, no npm deps — uses Node's built-in `node:wasi`). For each of the five headless crates it runs `cargo test -p <crate> --target wasm32-wasip1 --no-run`, parses every produced test binary, and executes each under a fresh `WASI` instance with the crate directory preopened (so relative golden paths resolve exactly as under cargo). The harness is deliberately **not** `wasm-bindgen-test`: the standard libtest binaries already compile to wasm32-wasip1 unchanged (no `#[wasm_bindgen_test]` rewrites, no new test framework in the tree), and Node WASI provides the real filesystem the golden tests need — QS4's deviation from its draft wording is recorded in §6.3.

**Platform fallbacks (AC3)** — all cfg-gated on `target_arch = "wasm32"`, all in the app/service layer only (Q2.2: zero forks in the five core crates):

| Area | Native | Web (S-019) |
|------|--------|-------------|
| File open | rfd system dialog → `FsStore::load` | DOM file picker (`webio::pick_ad_file`, `<input type=file>`) → content parsed through the seam's text parser (`open_from_picked`); result delivered next frame via `tick_web_open` |
| File save / Save As / export sinks | write to chosen path | in-memory `WebStore` (session lifetime) + browser download of the exact bytes (`webio::download`, object-URL anchor) — nothing lost when the tab closes |
| Custom collections | startup discovery of `*.adc` dir | no filesystem → no discovery; `ShapeCatalog::import_collection(name, text)` (ADR 009 v5) imports a user-supplied collection from pre-read text |
| Options persistence | JSON in platform config dir | same JSON in `localStorage` (`config::load_web/save_web`, key `ascii-diagram-drawer.config`; corrupt/absent → defaults, never fatal) |
| System dialogs (S-039 seam) | rfd + xdg-portal | `DialogResult::Unavailable` → inline prompt / DOM picker (unchanged S-039 fallback path) |
| PNG worker (ADR 012) | dedicated thread + bounded queue | inline executor (`PngWorker::for_target()` — same per-frame polling API, no threads on wasm) |
| Window controls / resize grips (S-027/S-040/B-004) | app-bar minimize/fullscreen/close + edge grips | not drawn — the browser chrome owns the window |

Deployment-target context: [§10.1](../arc42/10-deployment-view.md#101-deployment-concepts).

## Verification

- [x] **AC1** — `cd gui-app && trunk build --release` exits 0 and produces `gui-app/dist/`: `index.html` (2,567 B) + `gui-app-*.js` glue (159,663 B) + `gui-app-*_bg.wasm` (12,978,169 B). `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0. Tooling recorded above (Trunk 0.22 + eframe `WebRunner` + wasm-bindgen; lib/bbin split with `cdylib`).
- [x] **AC2** — `node scripts/wasm-core-tests.mjs`: the five headless crates' standard libtest binaries compiled to **wasm32-wasip1** and executed under Node WASI (per-crate cwd preopen for goldens). Result: 13 test binaries, **343 tests passed, 0 failed** — identical test count to the native run of the same crates; golden files read through WASI are byte-identical to native (same committed fixtures, e.g. `goldens_match_the_exported_bytes` ok). Harness is dependency-free Node ≥ 18 (`node:wasi`).
- [x] **AC3** — no feature reduction: the web build compiles the *same* app modules; only the platform surfaces above are cfg-gated (fallback table in Implementation notes, pointer to §10.1). Every gate listed there is either an existing seam fallback (dialogs → inline prompt, S-039) or a target-aware service (`WebStore`, `PngWorker::for_target()`) — the GUI code paths (canvas, gestures, sidebar, template editor, preview pane, export flows, options) are shared unmodified.
- [x] **AC4** — measured: JS + wasm = 159,663 + 12,978,169 = **13,137,832 bytes ≈ 12.5 MiB** (uncompressed). This **exceeds** the proposed 2 MB P3 budget — recorded as open risk **R1** in [§15](../arc42/15-risks.md). Caveat: this is an *upper bound* — `wasm-opt` was skipped because the devcontainer's egress proxy blocks the binaryen GitHub download (ADR 014); with optimization enabled the size drops substantially. The P3 budget stays "awaiting confirmation" (not a gate) per the AC.
- [x] **AC5** — gates green: `cargo fmt --check` clean; `cargo clippy --workspace --all-targets` **and** both wasm targets (`wasm32-unknown-unknown`, `wasm32-wasip1`) exit 0 (the wasm passes caught two native-invisible lints, both fixed: `let_unit_value` in `webio.rs`, `needless_return` in `begin_open`); `cargo test --workspace` **707 passed, 0 failed**; tarpaulin (Llvm) workspace **92.95%** ≥ 80%; `mkdocs build --strict` exits 0. Overview tracker updated: Q2/NFR1 verified at build+test level (wasm bundle builds + full core suite on wasm); manual browser smoke checklist below for a human.

**Manual browser smoke checklist (human — no browser exists in the devcontainer):**

1. `cd gui-app && trunk serve --release` (or `trunk build --release` then any static server over `dist/`, e.g. `python3 -m http.server`) → open the printed URL in a current Chrome/Firefox/Safari.
2. App renders: Material app bar with title, menu bar (File ▾ / View ▾ / Options ▾), sidebar with Built-in (Basic) + UML (Class/Component/Activity/Other) groups, empty canvas, preview pane. No console errors.
3. Click **box** in the sidebar → click canvas → box placed; drag it; double-click → type a label → Enter.
4. Second box; press an anchor of one and drag to the other's anchor → connector with arrowhead appears.
5. File ▾ **Save** → browser downloads the `.ad` (web store); reopen the downloaded file via File ▾ **Open…** (DOM picker) → diagram loads identically.
6. View ▾ **Colors…** → set background → canvas + preview follow; Options ▾ theme switch repaints live; reload the page → theme persists (localStorage).
7. File ▾ Export SVG… / Export PNG… → both download with correct content (PNG signature, SVG text matches canvas).
8. Template editor: ✎ on a built-in → edit art/anchors → save → new shape appears under Other and places on the canvas (in-session; collections are not persisted across reloads — no filesystem).
