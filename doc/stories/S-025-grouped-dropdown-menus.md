---
title: S-025-grouped-dropdown-menus
description: CR-002 — menu bar with grouped dropdown menus (FR6) (skeleton; ACs at implementation)
scope: gui-app
status: done
updated: 2026-10-31
---

# S-025: gui-app — menu bar with grouped dropdown menus (FR6)

## Why

FR6: the flat toolbar keeps growing as features land. A top menu bar of dropdown menus groups actions by category (e.g. File) — one-shot actions move into dropdowns, the visible surface stays minimal (NFR4), and new feature areas get their own menus without more top-level buttons.

## Dependencies

- none beyond done stories

## Acceptance criteria

### AC1: menu bar structure replaces the flat button row

The top toolbar is a **menu bar**: a **File ▾** dropdown holding every one-shot file action, grouped with separators — `Open…, Save, Save As…` | `Copy, Text .txt` | `Export SVG…, Export PNG…` — followed by the existing **Options ▾** menu (S-023). No flat action buttons remain in the top bar; the only non-menu control is the Scale group (a persistent live control, outside the dropdowns per CR-002's scope note).

### AC2: actions keep their exact behavior

Selecting a File item performs exactly what the old button did — path prompt of that kind / `save_requested` / clipboard copy with status message / `.txt` save / SVG or PNG export start (PNG disabled while a job is in flight). Ctrl+S still saves from anywhere except text inputs. Status-bar messages are unchanged.

### AC3: standard dropdown UX + extensibility

Clicking **File ▾** opens the menu; clicking an item performs it and closes the menu; Escape or a click outside closes without performing. The bar is extensible — a new feature area is one more `menu_button` call (Options already demonstrates coexistence) — no top-bar re-layout for future menus.

### AC4: no regressions

All existing tier-0b flows (open/save/save-as/copy/text export/export SVG/PNG, template editor, text tool, panning, scale, options menu) pass unchanged in behavior; the headless harness drives menus as click-menu-button → settle frame → click-item.

## Implementation notes

- `gui-app/src/app.rs` toolbar: the seven flat buttons become items inside one `ui.menu_button("File ▾", …)` with `ui.separator()` between the three groups; item bodies are the old button closures verbatim. The disabled-while-exporting PNG item uses `ui.add_enabled(!exporting, …)`.
- Test harness: new `Gui::click_menu_item(menu, item)` (open menu → settle → click item); `open_prompt(item)` routes through File so all 13 call sites keep working; the two direct-label tests (Escape-with-prompt, CopyText capture) gain a first File-menu step.

## Verification

All tier-0b (headless egui frame-pumping, no window/GPU — see the `gui-testing` skill). New tests in `gui-app/src/gui_tests.rs`:

| AC | Test | What it pins |
|----|------|--------------|
| AC1 | `toolbar_is_a_menu_bar_without_flat_action_buttons` | top bar = File ▾ + Options ▾ + Scale ×1; none of the seven action labels are flat buttons while menus are closed |
| AC1/AC3 | `file_menu_lists_the_grouped_actions` | opening File ▾ lists all seven items in group order (Open…/Save/Save As… → Copy/Text .txt → Export SVG…/Export PNG…) |
| AC2/AC3 | `file_menu_item_performs_and_closes_the_menu` | File ▾ → Save As… opens the prompt and closes the menu (items gone from the frame) |
| AC3 | `escape_closes_the_file_menu_without_performing` | Escape on an open File menu closes it, no prompt opened |
| AC2 | `export_png_item_is_disabled_while_an_export_runs` | with a held in-flight job (S-015 seam, deterministic — no worker race) the Export PNG… item is listed but inert; after the job completes the bytes are written, the state clears, and the item opens a prompt again |

AC2/AC4 regressions: all pre-existing tier-0b flows now drive the File menu via the new `Gui::click_menu_item` helper — `open_prompt(item)` routes through it, so every open/save/export test (13 call sites) exercises the real dropdown path; `ctrl_s_after_edit_rewrites_the_current_file` pins that Ctrl+S still saves with the menu closed (the shortcut fires outside the menu closure); the two direct-label tests (`escape_while_prompt_opens…`, the CopyText capture) open File ▾ first; the template-editor Save tests now assert exactly one visible "Save" (the window's). Visual evidence: `GUI_DUMP=1 cargo test -p gui-app visual_dump` → `/tmp/gui-dump/09-file-menu.png` (dropdown with separators over the canvas) and `01-empty.png` (compact menu bar, no flat row).

Gates: `cargo fmt --all` clean; `cargo clippy --workspace --all-targets` 0 warnings; `cargo test --workspace` green (474 passed); `cargo tarpaulin --engine Llvm --workspace` 94.06% ≥ 80% target. 
