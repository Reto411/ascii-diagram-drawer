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

**None at runtime.** The application is fully offline on both targets:

| Aspect | Native | Web |
|--------|--------|-----|
| Server / API calls | none | none — the bundle is self-contained static files; no XHR/fetch beyond loading its own wasm module |
| External services (CDN, fonts) | none | none — no webfonts or CDN links in `gui-app/index.html`; eframe uses its bundled font stack + the system monospace family for the canvas |
| Network use at all | build time only (crates.io) | build time only (crates.io + binaryen download when `wasm-opt` is enabled) |

Consequences: no telemetry, no update checks, no network failure modes; the security surface is local files only ([§11](11-cross-cutting-concerns.md)).

## 10.3 Deployment Model

> *Build pipeline: build → test → package → publish. CI tooling.*

- **Isolated build environment:** the podman devcontainer (`.devcontainer/`) mounts only this repo at `/workspace` — builds/tests never depend on host state. All quality gates run inside it.
- **Gates per change** (the de-facto pipeline): `cargo fmt --check` → `cargo clippy --all-targets` (`clippy::all` deny) → `cargo test --workspace` (714+ tests) → `cargo tarpaulin --engine Llvm --workspace` (≥ 80%) → wasm gates (`cargo check -p gui-app --target wasm32-unknown-unknown`; core suite on `wasm32-wasip1` under Node WASI via `scripts/wasm-core-tests.mjs`) → `mkdocs build --strict`. Benchmarks: `cargo bench -p render-pipeline` / `-p exporters` (criterion, native-only).
- **Packaging:** native = `cargo build --release -p gui-app` → single binary; web = `cd gui-app && trunk build --release` → static files in `gui-app/dist/` (index.html + JS glue + wasm), deployable to any static host over http(s) — not `file://` (wasm fetch + ES-module CORS).
- **Publishing:** not automated yet — releases are git tags plus the two artifacts above; no CI runner is configured in-repo (the devcontainer + gates are the reproducible environment).

## 10.4 Operation & Maintenance

> *Monitoring, logging, updates, backups (as applicable).*

- **No service to operate:** both targets run entirely on the user's machine/browser — nothing to monitor, no logs server-side, no telemetry.
- **User data:** documents are plain-text `.ad`/`.adc` files on disk (native) or in an in-memory store with browser-download (web); options are a small JSON file in the platform config dir (native) / `localStorage` (web). Backups = ordinary file/VCS backups; nothing tool-specific.
- **Updates:** replace the binary / redeploy the static bundle. No auto-update mechanism in v1. Corrupt or missing options files fall back to defaults at load (never fatal).

## 10.5 Release Strategy

> *Versioning scheme, release cadence, deprecation policy.*

- **Application version:** Cargo workspace version `0.1.0` (pre-1.0). From 1.0 onward: SemVer; breaking changes to the *application* bump major.
- **File formats are versioned independently** of the application: `.ad` diagrams carry a header version (currently **1.1**) and `.adc` collections likewise; minor-version extensions must stay backward-readable, pinned by parser tests (e.g. 1.0 files remain parseable after the 1.1 extensions). A format bump is an API decision recorded as an ADR version change ([ADR 004](../adr/004-diagram-file-format.md), [ADR 007](../adr/007-user-defined-diagram-objects.md)).
- **Cadence:** story-driven — releases cut when the board's v1 scope is done; no fixed calendar cadence pre-1.0.
- **Deprecation:** none before 1.0; after, format features are deprecated for at least one minor version with a parser warning before removal.
