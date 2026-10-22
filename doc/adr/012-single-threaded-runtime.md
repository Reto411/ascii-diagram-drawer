---
title: "012 - single-threaded-runtime"
description: v1 runs on one process / one thread — no async runtime, no worker threads; all model/render/IO work inline on the UI thread; optional PNG worker is a pre-defined, confined escape hatch
scope: architecture / runtime
status: proposed
version: 1
updated: 2026-10-22
---

# ADR 012: Single-Threaded Runtime Model

## Context

[§9 Runtime View](../arc42/09-runtime-view.md) needs a concurrency model before the runtime scenarios can be specified — because *communication between components depends on the threading decision*: in one thread, all communication is direct synchronous calls; add threads and you get channels/shared state to design, test, and keep deterministic.

Constraints in play:

- **Scale (P1/P2):** 500 objects / 200×100 cells (≈20k cells), first render < 100 ms, drag feedback ≤ 16 ms/frame. The whole pipeline is O(cells) — realistically low single-digit milliseconds on reference hardware.
- **GUI framework ([ADR 008](008-gui-framework-choice.md)):** egui/eframe is immediate mode — the idiomatic pattern is all application logic inline in the event-loop thread. eframe is not async-native.
- **Portability (Q2):** one codebase → native *and* wasm32. `wasm32-unknown-unknown` has **no threads at all** (`std::thread::spawn` panics) — any threading design must degrade to single-threaded on the web target anyway.
- **Determinism (Q1):** fewer moving parts, fewer ways for iteration/scheduling order to leak into output.

Decision forced: how much concurrency does v1 actually need?

## Decision

**v1 is single-process, single-thread.** Everything — model mutation, rendering, `.ad` parse/serialize, SVG export, PNG rasterization — runs inline on the UI thread inside the eframe event loop.

1. **No async runtime** (no tokio/async-std), **no worker threads**, no channels, no shared mutable state, no locks.
2. **Component communication = direct synchronous function calls passing value types** — the runtime graph is exactly the static dependency DAG of [§8.3](../arc42/08-building-block-view.md). There is no separate "runtime coupling" to design or test.
3. **Pre-defined escape hatch (not a fork):** if profiling ever shows PNG rasterization exceeding the frame budget, add *one* optional worker confined to `core-services` — one-shot job submission + result return, no shared state, main thread stays responsive via completion on next idle frame; on wasm it compiles to inline execution. This is a local change inside the application layer, not an architectural revision (see [ADR 009](009-ui-api-service-layer.md) — the seam already isolates exactly where such a worker would live).
4. Headless tests stay trivially single-threaded: call the functions directly (Q5.2/T5).

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Async runtime (tokio/async-std) for file IO + PNG | eframe is not async-native (would need a bridge); there is no concurrent IO to overlap — files are small local text; adds a second execution model to reason about on *both* targets for zero measured benefit at P1/P2 scale |
| Worker threads from day one (parse, render, export offloaded) | YAGNI at 20k-cell scale; breaks wasm parity unless feature-gated everywhere; introduces channels/shared state that fight Q1 determinism and complicate T5/T7 testing — none of which is justified by any current measurement |
| `rayon` parallel rendering (per-row grid resolution) | O(20k cells) with tiny per-cell work — parallelization overhead dominates at this size, and it creates pressure to make the resolve stage order-independent in ways that complicate Q1's fixed-priority determinism |

## Consequences

- **Positive:** one mental model; runtime communication is *identical* to static coupling (direct calls, value types) — nothing new to design, review, or test; wasm and native behave identically; frame budget (P2) becomes a pure computation problem verified by QS12 benchmarks; the escape hatch keeps future options open without rework.
- **Negative / costs:** a pathologically slow PNG rasterizer would block the UI on export — mitigated by the pre-defined worker escape hatch and by choosing a fast pure-Rust rasterizer (open item, §8.5); no overlap of save+render possible (irrelevant at this scale).
- **Follow-ups:** QS12 `criterion` benchmarks must be in place before any "it feels slow" claim triggers the worker discussion; PNG rasterizer choice ([§8.5](../arc42/08-building-block-view.md) open item) should explicitly consider single-thread cost on wasm.

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v1 (2026-10-22):** initial decision — single-thread v1; no async/workers/channels; communication = direct calls per §8.3 DAG; optional confined PNG worker as pre-defined escape hatch (inline fallback on wasm).
