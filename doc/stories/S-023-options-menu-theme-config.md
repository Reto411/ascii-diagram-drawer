---
title: S-023-options-menu-theme-config
description: CR-005 — Options menu: theme selection (light / dark / native, default) + config persistence across sessions (FR13)
scope: gui-app
status: done
updated: 2026-10-31
---

# S-023: gui-app — Options menu: theme selection + config persistence (FR13)

## Why

FR13: the UI was fixed to a dark theme with no way to adapt it. An Options menu becomes the standard home for user preferences; theme (light / dark / native-inherited, default) is its first option, persisted in the application's config file across sessions.

## Dependencies

- none beyond done stories

## Acceptance criteria

Written from [CR-005](../crs/CR-005-options-menu-theme-config.md)'s "Requested change" + the §14 FR13 wording.

- **AC1 — Options menu with theme choices:** the toolbar offers an **Options** dropdown listing the three theme choices — **Light** (mainly white), **Dark** (mainly dark), **Native (system)** (inherited from the window manager / system appearance, the default). The current choice is marked.
- **AC2 — live and at startup:** a selection applies immediately (the context switches theme in the same frame), and the stored theme is applied at startup before the first paint — no flash of unstyled frames. Native follows the system appearance where the platform provides it (winit's OS query on native, `prefers-color-scheme` on the web — both fed by eframe 0.37 through `RawInput::system_theme`) and falls back to dark where it is unknown, as CR-005 explicitly allows.
- **AC3 — persistence:** the selection is written to the application's config file (JSON in the platform config directory) and restored on the next launch. Loading is never fatal — a missing, unreadable or corrupt file yields the defaults; a save failure is reported in the status bar, not panicked.

## Implementation notes

- `gui-app/src/config.rs` (new) — `Theme` (light/dark/native, serde lowercase) + `AppConfig { theme }` with tolerant `load`/`save`; `default_config_path()` via the `directories` crate (`…/ascii-diagram-drawer/config.json`). Unknown fields are ignored, absent ones default, so files from other app versions still load.
- `gui-app/src/app.rs` — `DrawingApp` gains `config`, `config_path`, `applied_theme`; `create_app` loads the config; `show()` applies the theme once per change (startup included); `set_theme()` applies live + persists + reports via the status bar; the toolbar gets the **Options ▾** menu with radio choices.
- egui 0.37's native theme system does the heavy lifting: `Theme::preference()` maps to `egui::ThemePreference::{Light, Dark, System}`, and egui resolves `System` against `RawInput::system_theme` (eframe feeds it on every target) with a dark fallback — better than the CR assumed ("best-effort per target"), no extra detection code.
- `gui-app/src/main.rs` — the hardcoded `Visuals::dark()` at startup is gone; the app applies its configured theme on the first frame instead.
- New deps (gui-app only): `serde` (+derive), `serde_json`, `directories`.

## Verification

Tier 0 (headless unit) + tier 0b (full-app egui frame pumping, `gui_tests.rs`):

- `config::tests::*` (9 tests) — default is native; all themes round-trip through JSON; missing/corrupt/unknown-field files load the defaults without failing; save creates parent dirs; `Native` maps to `ThemePreference::System` and resolves to dark without a system theme.
- `gui_tests::options_menu_selects_light_theme_and_persists_it` — opens **Options ▾**, asserts all three choices are listed, clicks **Light**: the context theme is Light, the app state holds it, the config file contains `"theme": "light"`, and the status bar reports "Theme: light (saved)".
- `gui_tests::the_stored_theme_is_applied_at_startup` — a pre-written `{"theme": "light"}` config yields a Light context after the first frame, before any click.
- `gui_tests::the_default_native_theme_renders_dark` — the default (native) app resolves to dark in the headless harness (no system appearance fed), i.e. CR-005's allowed fallback.
- Full regression: `cargo test --workspace` green (160 gui-app tests); `cargo clippy --workspace --all-targets` clean; workspace coverage via `cargo tarpaulin --engine Llvm --workspace` 94.76% (≥ 80% target).
- Deterministic goldens: the headless harness feeds no system appearance, so native resolves to dark exactly as the old hardcoded theme did — settled frames are unchanged for the visual tiers (opt-in `GUI_DUMP=1`; no committed goldens exist).
