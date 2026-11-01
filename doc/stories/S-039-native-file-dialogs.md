---
title: S-039-native-file-dialogs
description: CR-001 — system file dialogs for open/save/export + Save As .txt (FR2/FR4/FR5)
scope: gui-app
status: in-progress
updated: 2026-11-01
---

# S-039: gui-app — system file dialogs for open/save/export + Save As .txt (FR2/FR4/FR5)

## Why

FR2/FR4/FR5: Open / Save / Save As use a system file dialog; Save As offers what to save — the `.ad` document or the rendered `.txt`; Export becomes one system dialog with an SVG/PNG format choice, replacing the two separate export buttons + inline path prompts. Supersedes S-014's 'no native file dialog in v1' note (native targets); must not regress the S-019 web path.

## Dependencies

- S-025 (preferred) — actions live in the File menu after the restructure, avoiding a second move

## Acceptance criteria

From [CR-001](../crs/CR-001-native-file-dialogs.md) "Requested change" + §14 FR2/FR4/FR5/NFR1 wording.

- **AC1 (Open, FR4).** File ▾ → `Open…` opens a system file dialog on native targets with an available session; the chosen path goes through the existing open flow — success replaces the model and remembers the file, parse/IO errors keep the previous model and show a notice (S-014 RS2/RS5); cancelling changes nothing.
- **AC2 (Save, FR4).** File ▾ → `Save` (and Ctrl+S) opens a system save dialog prefilled with the current file when a document is open; the chosen path writes the `.ad` document through the existing save flow — a lossy load still goes through the confirmation dialog first.
- **AC3 (Save As, FR2/FR4).** File ▾ → `Save As…` opens one system dialog in which what to save can be selected: the diagram document (`.ad`) or the rendered text output (`.txt`). `.ad` runs the existing save-as flow (new current file, lossy-load confirmation); `.txt` writes exactly the canvas bytes (WYSIWYG) to the chosen path without changing the current file. The standalone `Text .txt` menu item is removed — its function lives here.
- **AC4 (Export, FR5).** One File ▾ → `Export…` item replaces `Export SVG…`/`Export PNG…`; it opens one system dialog in which the export format (SVG or PNG) can be selected. SVG writes the exact exporter bytes synchronously; PNG goes through the non-blocking worker with the in-flight "exporting" state (S-015 AC2/RS4).
- **AC5 (web/fallback, NFR1).** On wasm targets and on native hosts without a usable D-Bus session no dialog is attempted: `Open…` keeps today's inline prompt, `Save`/Ctrl+S keep today's direct-write behavior, and `Save As…`/`Export…` first show an in-app format choice (Diagram `.ad` / Text `.txt`; SVG / PNG) followed by the prefilled inline prompt.

## Implementation notes

- **New module `gui-app/src/dialogs.rs`** — the dialog seam + backend:
  - `DialogKind { Open, Save, SaveAs, Export }`, `DialogResult { Chosen(PathBuf), Cancelled, Unavailable(String) }`, and the `FileDialogBackend` trait (`show(kind, default_file)`). The app depends only on this trait.
  - `RfdBackend` — production implementation: **rfd 0.17** with the `xdg-portal` feature (GTK-free; D-Bus/`zbus` already in the tree for other reasons). Availability is probed by the presence of `DBUS_SESSION_BUS_ADDRESS` before any dialog call, so a headless native host degrades to `Unavailable` instead of hanging. `Open` → open-mode picker; the other three → save-mode pickers with the prefilled file name (rfd `set_file_name`) and directory.
  - Pure helpers: `save_mode_for(kind)` (Save/SaveAs/Export all use save mode), `prefill_for(kind, current_file)` (Save/SaveAs → current file; Export → empty), and `extension_for(choice)` for the in-app format choice. Unit-tested directly.
- **`DrawingApp`** (`app.rs`):
  - New field `dialog_backend: Box<dyn FileDialogBackend>` + constructor `with_dialog_backend` (default = `RfdBackend`). The tier-0b harness installs a fake backend, so no test can open a real dialog.
  - New enum variants / flows: `PathPrompt::SaveAsText` (fallback twin of the dialog's `.txt` choice), `FormatChoice { SaveAs, Export }` state for the in-app format window, and `write_text_to(path)` (native writes the exact canvas bytes via `fileio::write_text_file`; wasm stubs with a notice until S-019).
  - Handlers: `begin_open` / `begin_save` / `begin_save_as` / `begin_export` each ask the backend first — `Chosen` routes into the existing open/save-as/export flows (no behavior change there), `Cancelled` is a no-op, `Unavailable` falls back (Open → inline prompt; Save → direct write as today; Save As/Export → format-choice window → prefilled inline prompt).
  - The old `save_text()` method and its `Text .txt` menu item are removed — the function lives in the Save As dialog's `.txt` choice (AC3) and its fallback twin.
- **File menu restructure** (`appbar.rs`): `Open… | Save | Save As… | Copy | Export…` — flat, one level. The one-item `Output` submenu and the two-item `Export` submenu are gone (their surviving members were flattened); `Export…` is disabled while a PNG job is in flight (S-015 AC2).
- **In-app format-choice window** (`appbar.rs`): a small `egui::Window` listing the two options per action; picking one starts the matching prefilled inline prompt. Shown only when the backend reported `Unavailable`.
- **Ctrl+S** now routes through `begin_save` (dialog → fallback direct write), keeping today's headless behavior intact.

## Verification

- Tier 0: `dialogs.rs` unit tests — prefill/extension/save-mode decision tables; availability probe (env var gate) with the session bus absent (skipped when a real bus is present).
- Tier 0b (`gui_tests.rs`, fake backend via `Gui::new`):
  - AC1: `open_dialog_chosen_path_replaces_the_model` (chosen path → model replaced, current file remembered, `(Open, None)` request recorded), `open_dialog_cancelled_changes_nothing`.
  - AC2: `save_dialog_is_prefilled_and_writes_the_chosen_path` (request recorded as `(Save, Some(current))`; chosen path written; status "Saved"), `save_dialog_cancelled_keeps_the_document`.
  - AC3: `save_as_dialog_txt_writes_text_output_without_touching_the_document` (`.txt` == exact canvas bytes; current file untouched), `save_as_dialog_ad_runs_the_save_as_flow` (current file moves to the chosen path).
  - AC4: `export_dialog_svg_writes_the_exporter_bytes`, `export_dialog_png_runs_the_non_blocking_worker` (job in flight → pumped to completion → PNG magic bytes).
  - AC5: `fallback_save_as_offers_both_options_and_txt_writes` (Unavailable → both options listed; `.txt` choice opens the sibling-prefilled `SaveAsText` prompt; OK writes exact canvas bytes), `fallback_export_offers_both_formats_with_prefill` (SVG/PNG choices open their prompts with sibling prefills).
  - Menu structure: `file_menu_lists_the_grouped_actions_flat` (flat order, no `Output`/`Export` submenus, no `Text .txt`); `file_menu_item_performs_and_closes_the_menu` updated for the format-choice window; `export_png_item_is_disabled_while_an_export_runs` retargeted to the single `Export…` item.
  - All pre-S-039 prompt tests keep passing unchanged (the fake's default `Unavailable` reproduces today's fallback flows); `open_prompt` helper now clicks through the format choice for Save As/Export.
- Gates: `cargo fmt`, `cargo clippy --workspace --all-targets` (0 warnings/errors), `cargo test --workspace` green (gui-app 295 tests), `cargo tarpaulin --engine Llvm --workspace` = **92.65%** (target 80%).
- Web path: no rfd call is compiled on wasm32 (`#[cfg(not(target_arch = "wasm32"))]`); the fallback wording of AC5 applies unchanged once S-019 lands.

## Commit

 
