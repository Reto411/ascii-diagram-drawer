---
title: "arc42-11-cross-cutting-concerns"
description: concerns applying across building blocks — errors, logging, security, config, i18n/Unicode, a11y
scope: architecture
status: stable
updated: 2026-11-02
---

# 11. Cross-cutting Concerns

> *Concerns that apply across building blocks.*

| Concern | Approach |
|---------|----------|
| **Error handling** | Typed domain errors per bounded context — `ModelError` (invariant violations), `ParseError` with line/column position (`.ad`/`.adc`), `RenderError`, `ExportError`, `LookupError`/`RegionSetError`/`ResizeError` (catalog) — surfaced through the seam as `ui-api` error types ([ADR 009](../adr/009-ui-api-service-layer.md)). Core crates **never panic on invalid input**; every fallible API returns `Result`. The GUI translates errors into status-bar notices and file-IO dialogs; nothing crosses into the windowing layer unhandled. |
| **Logging / observability** | Deliberately absent in v1: a fully offline desktop tool has no service to observe, and there is no log data to collect. Errors surface in the UI (status bar, dialogs); diagnostics for development are test output. A logging framework would be added only if a server-side component ever appears ([§10.2](10-deployment-view.md#102-network-topology)). |
| **Security (input validation)** | No runtime network ⇒ no XSS/injection surface beyond local files. Validation is by construction: strict parsers with positioned, typed errors for `.ad`/`.adc` (no `unwrap` on user input anywhere in the core); geometry operations are total functions over checked value types; file access only through OS-native dialogs (rfd) — the app never scans directories unsolicited; options JSON is validated at load and falls back to defaults when missing or corrupt. |
| **Configuration** | One `AppConfig` (theme, recent colors, sidebar expansion state, …) persisted as JSON: platform config dir via `directories` (native), `localStorage` (web). Loaded once at startup, saved on change; unknown keys ignored, missing file ⇒ built-in defaults. Diagram-level presentation settings (colors, tips, bends) live *in the document*, not in app config — they travel with the file (NFR2). |
| **Internationalization / Unicode** | Output text is Unicode box-drawing characters — rendering requires a monospace font covering them (system default; eframe's bundled fallbacks keep the canvas consistent across OSes, [§10.3](10-deployment-view.md#103-deployment-model)). UI strings are English-only in v1 (no i18n framework). All geometry is integer cell coordinates — no floating-point or locale-dependent formatting in the data path; hex colors are case-insensitive on input, canonical lowercase on output. |
| **Accessibility** | Menus, sidebar rows, and dialogs are keyboard-operable through egui's default focus/interaction model. The canvas itself is inherently visual (it *is* an ASCII-art editor) — full a11y support (screen-reader descriptions of the diagram) is out of v1 scope and tracked as future work. Color contrast follows the Material palette tokens (dark/light themes, CR-005/CR-006); overlays are high-contrast against the fixed canvas background. |
