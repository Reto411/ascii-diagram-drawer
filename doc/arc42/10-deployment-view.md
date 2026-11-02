---
title: "arc42-10-deployment-view"
description: deployment concepts, network topology, build pipeline, operations, release strategy
scope: architecture
status: draft
updated: 2026-11-02
---

# 10. Deployment View

> *How the system is deployed and operated.*

## 10.1 Deployment Concepts

Two deployment targets, one codebase (NFR1/Q2, ADR 008):

| Target | Build | Runtime | State |
|--------|-------|---------|-------|
| Native desktop (Windows / macOS / Linux) | `cargo build -p gui-app` → single binary (undecorated Material window; app-bar controls S-027/S-040, edge/corner resize B-004) | winit + wgpu | options JSON in the platform config dir (`directories`); documents on disk via rfd dialogs (S-039) |
| Web bundle (any modern browser) | `cd gui-app && trunk build --release` → static files in `gui-app/dist/` (index.html + JS glue + wasm), servable from any static host over http(s) — not `file://` (wasm fetch + ES-module CORS) | wasm32-unknown-unknown, eframe `WebRunner`, wgpu-in-webgl | options JSON in `localStorage`; documents in an in-memory store with browser-download on save (no persistent filesystem) |

The web build is the **same app code** as native (`gui-app` lib + thin entry points); platform differences are cfg-gated at the service/app seams only (Q2.2 — zero forks in the core crates). The full list of web fallbacks lives in [S-019's Implementation notes](../stories/S-019-wasm-web-milestone.md#implementation-notes) (file IO, options persistence, dialogs, PNG worker, window controls).

Bundle size: measured 13,137,832 B uncompressed/unoptimized at S-019 — see risk [R1](15-risks.md) against the proposed P3 budget. The core test suite runs unmodified on wasm (libtest binaries → `wasm32-wasip1`, Node WASI, `scripts/wasm-core-tests.mjs`).

## 10.2 Network Topology

> *Client/server boundaries, APIs called, external services (fonts, CDNs).*

```
[placeholder]
```

## 10.3 Deployment Model

> *Build pipeline: build → test → package → publish. CI tooling.*

- [TBD]

## 10.4 Operation & Maintenance

> *Monitoring, logging, updates, backups (as applicable).*

- [TBD]

## 10.5 Release Strategy

> *Versioning scheme, release cadence, deprecation policy.*

- [TBD]
