---
title: "012 - responsive-ui-threading"
description: the UI thread is never blocked by application work — frame-critical work runs inline on the dedicated UI thread, non-frame work (PNG rasterization) on a worker thread (native) with inline fallback on wasm; no separate process (impossible on the web target)
scope: architecture / runtime
status: proposed
version: 2
updated: 2026-10-22
---

# ADR 012: Responsive UI — Dedicated UI Thread, Worker Offload for Non-Frame Work

## Context

[§9 Runtime View](../arc42/09-runtime-view.md) needs a concurrency model before the runtime scenarios can be specified — because *communication between components depends on the threading decision*: in one thread, all communication is direct synchronous calls; add threads and you get channels/shared state to design, test, and keep deterministic.

Constraints in play:

- **UI responsiveness is the top UX priority** (stated requirement): an unresponsive app is unacceptable — no application work may freeze input handling or painting.
- **Scale (P1/P2):** 500 objects / 200×100 cells (≈20k cells), first render < 100 ms, drag feedback ≤ 16 ms/frame. The interactive pipeline is O(cells) — realistically low single-digit milliseconds on reference hardware.
- **GUI framework ([ADR 008](008-gui-framework-choice.md)):** egui/eframe is immediate mode; application logic runs where `update()` is called (the UI thread). eframe is not async-native.
- **Portability (Q2):** one codebase → native *and* wasm32. `wasm32-unknown-unknown` has **no threads** (`std::thread::spawn` panics), and browsers provide **no subprocesses at all** — any threading design must degrade to single-threaded on the web target, and a separate-process design is categorically impossible there.
- **Determinism (Q1):** fewer moving parts, fewer ways for scheduling order to leak into output.

The question: how do we guarantee a responsive UI without adding runtime complexity that fights Q1/Q2?

## Decision

**The UI has its own dedicated thread and is never blocked by application work.** Work is split by one rule:

1. **Frame-critical work stays inline on the UI thread** — anything that must land within the current frame: input handling, model mutation (direct aggregate manipulation, [ADR 009](009-ui-api-service-layer.md)), grid rendering during gestures, painting. Moving per-frame work to another thread cannot make it faster; it only adds latency and synchronization. P2 (≤16 ms/frame) is enforced by benchmark ([QS12](../arc42/06-quality-model.md)), not by threading.
2. **Non-frame work runs on a worker thread** — anything the user waits for but that need not complete this frame: in v1 exactly one operation, **PNG rasterization**. Pattern: one-shot job submission + result return via a bounded channel (`std::sync::mpsc`), polled on subsequent frames; the UI shows an "exporting" state meanwhile. No shared mutable state — the channel transfers ownership of the job and the result.
3. **Wasm fallback:** on `wasm32-unknown-unknown` the worker compiles to inline execution (feature-gated) — the web build accepts that a pathologically slow rasterizer can block, because no threads exist on the target; choosing a fast pure-Rust rasterizer ([§8.5](../arc42/08-building-block-view.md) open item) keeps this risk theoretical.
4. **No separate process** for UI or core — see alternatives.
5. Headless tests stay trivial: call the functions directly (Q5.2/T5); the worker path is tested through the same job/result API with a synchronous executor, so both execution paths share one code path and the same golden tests.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Everything single-threaded, inline (this ADR's v1) | Violates the responsiveness priority: a slow PNG rasterizer would freeze input and painting for its full duration. Superseded by v2 after review — "nothing is worse than an unreactive app" |
| UI in a separate process (native) | Impossible on the web target — browsers provide no subprocesses (Q2 one codebase); on native it adds IPC + serialization for work that takes at most tens of ms; two processes to build, test, and ship for zero measured benefit at P1/P2 scale |
| Async runtime (tokio/async-std) for file IO + PNG | eframe is not async-native (would need a bridge); there is nothing to overlap besides the one PNG job; adds a second execution model on *both* targets |
| Worker threads for everything (parse, render, export offloaded) | YAGNI at 20k-cell scale; interactive work must be inline anyway (frame deadline); more channels/shared state fight Q1 determinism and complicate T5/T7 — only the one operation with unbounded cost (rasterization) earns a worker |
| `rayon` parallel rendering (per-row grid resolution) | O(20k cells) with tiny per-cell work — parallelization overhead dominates at this size, and it pressures the resolve stage to be order-independent in ways that complicate Q1's fixed-priority determinism |

## Consequences

- **Positive:** UI responsiveness is guaranteed by construction for all v1 operations except on wasm (where no alternative exists); the interactive hot path stays a pure, benchmarkable, single-threaded computation (QS12); the worker is confined to `core-services` behind the existing seam ([ADR 009](009-ui-api-service-layer.md)) — GUI code only ever sees an "exporting" state and a result; determinism preserved (ownership transfer, no shared mutable state, rendering stays fixed-priority single-threaded).
- **Negative / costs:** one job state machine + channel + "exporting" UI state to build and test; two execution paths for PNG (worker/inline) that must stay behaviorally identical — enforced by sharing golden tests; wasm retains the theoretical freeze risk.
- **Follow-ups:** QS12 `criterion` benchmarks in place before any further offload discussion; PNG rasterizer choice ([§8.5](../arc42/08-building-block-view.md) open item) should explicitly consider single-thread cost on wasm; if a second non-frame operation ever appears, it reuses the same job/result mechanism — no new patterns.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v2 (2026-10-22):** decision changed after review. v1's "everything inline; worker only as escape hatch" is superseded: the UI thread is now *guaranteed* never blocked by application work, and PNG rasterization runs on a worker thread **by default** (native) with inline fallback on wasm. Rationale: responsiveness stated as top UX priority — an unreactive app is unacceptable. "UI in a separate process" evaluated and rejected: impossible on the web target (no subprocesses in browsers), IPC overkill at this scale on native. Frame-critical work remains inline — threading cannot speed up per-frame work; P2 stays benchmark-enforced. File renamed from `012-single-threaded-runtime.md` to match the current decision.
- **v1 (2026-10-22):** initial decision — single-thread v1; no async/workers/channels; communication = direct calls per §8.3 DAG; optional confined PNG worker as pre-defined escape hatch (inline fallback on wasm).
