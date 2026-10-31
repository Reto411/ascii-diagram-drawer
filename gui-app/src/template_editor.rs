//! Template editor state (S-017 AC1/AC2) — pure data + operations, unit-tested
//! headlessly. The egui drawing in `app.rs` stays thin; every user-visible decision
//! (which input is rejected, which anchor name a cell gets, what blocks saving) lives
//! here.
//!
//! The alphabet is svgbob's input set: printable ASCII U+0020–U+007E, where space is
//! the empty cell (ADR 001). Anything else is rejected with an inline hint (AC1).

use diagram_model::geometry::CellPos;
use ui_api::{AnchorInfo, EdgeRuleSpec, NewTemplate, ParityCharsSpec, ResizeRuleSpec, ShapeInfo};

/// The svgbob alphabet: printable ASCII U+0020–U+007E (space = empty cell).
#[must_use]
pub fn is_valid_cell_char(c: char) -> bool {
    matches!(c as u32, 0x20..=0x7e)
}

/// A template being edited: the grid, its declared anchors, and the last rejection.
#[derive(Debug, Clone)]
pub struct TemplateDraft {
    /// The collection the template belongs to (its `.adc` file on save).
    pub collection: String,
    /// The template name — a single token, unique within the collection.
    pub name: String,
    /// Exactly `height` rows of exactly `width` chars each; space = empty cell.
    pub rows: Vec<String>,
    /// Declared anchors (unique names, one per cell in the GUI).
    pub anchors: Vec<AnchorInfo>,
    /// The resize toolset (S-024 AC6) — off by default; when on, saving requires a
    /// complete, valid rule set.
    pub resize: ResizeRulesDraft,
    /// Inline hint for the last rejected input (AC1) — cleared on the next edit.
    pub rejection: Option<String>,
}

impl TemplateDraft {
    /// A fresh blank template of `width × height` cells (all spaces).
    #[must_use]
    pub fn new(
        collection: impl Into<String>,
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Self {
        let rows = vec![" ".repeat(width as usize); height as usize];
        Self {
            collection: collection.into(),
            name: name.into(),
            rows,
            anchors: Vec::new(),
            resize: ResizeRulesDraft::default(),
            rejection: None,
        }
    }

    /// Loads an existing template for editing (S-017 AC3): the catalog's preview rows
    /// become the grid (right-padded to the shape's width), anchors as declared.
    #[must_use]
    pub fn from_existing(info: &ShapeInfo, anchors: Vec<AnchorInfo>) -> Self {
        let width = info.size.width as usize;
        let mut rows: Vec<String> = info
            .preview
            .iter()
            .take(info.size.height as usize)
            .map(|row| {
                let mut s = row.clone();
                for _ in 0..width.saturating_sub(s.chars().count()) {
                    s.push(' ');
                }
                s
            })
            .collect();
        // A missing/short preview (e.g. a fake catalog) still yields the full grid.
        while rows.len() < info.size.height as usize {
            rows.push(" ".repeat(width));
        }
        let collection = info.collection.clone().unwrap_or_default();
        Self {
            collection,
            name: info.name.clone(),
            rows,
            anchors,
            resize: ResizeRulesDraft::default(),
            rejection: None,
        }
    }

    /// The grid's width in cells.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.rows
            .first()
            .map(|r| r.chars().count() as u32)
            .unwrap_or(0)
    }

    /// The grid's height in cells.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.rows.len() as u32
    }

    /// Sets the character at `(x, y)`; rejects non-alphabet input with an inline hint
    /// (AC1) and out-of-bounds cells silently. Returns `true` when accepted.
    pub fn set_cell(&mut self, x: u32, y: u32, c: char) -> bool {
        if !is_valid_cell_char(c) {
            self.rejection = Some(format!(
                "'{c}' is not in the svgbob alphabet (printable ASCII) — cell unchanged"
            ));
            return false;
        }
        let Some(row) = self.rows.get_mut(y as usize) else {
            return false;
        };
        let mut chars: Vec<char> = row.chars().collect();
        if (x as usize) >= chars.len() {
            return false;
        }
        chars[x as usize] = c;
        *row = chars.into_iter().collect();
        self.rejection = None;
        true
    }

    /// Clears the cell at `(x, y)` (sets it to the empty-cell space).
    pub fn clear_cell(&mut self, x: u32, y: u32) {
        if let Some(row) = self.rows.get_mut(y as usize) {
            let mut chars: Vec<char> = row.chars().collect();
            if (x as usize) < chars.len() {
                chars[x as usize] = ' ';
                *row = chars.into_iter().collect();
            }
        }
        self.rejection = None;
    }

    /// Toggles the anchor at `(x, y)`: adds one (auto-named) or removes it. Returns
    /// the new anchor's name when added, `None` when removed.
    ///
    /// Auto-naming: an edge midpoint gets the standard built-in name (`n`/`s`/`w`/`e`),
    /// anything else a position token `{x}_{y}` — deterministic, unique per cell, and
    /// consistent with how connectors reference the four default anchors (FR11).
    pub fn toggle_anchor(&mut self, x: u32, y: u32) -> Option<String> {
        let pos = CellPos::new(x, y);
        if let Some(idx) = self.anchors.iter().position(|a| a.position == pos) {
            self.anchors.remove(idx); // removed: no new name
            self.rejection = None;
            return None;
        }
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let name = Self::anchor_name_for(x, y, self.width(), self.height());
        self.anchors.push(AnchorInfo {
            name: name.clone(),
            position: pos,
        });
        self.rejection = None;
        Some(name)
    }

    /// The auto-name for an anchor at `(x, y)` on a `w × h` grid (see [`toggle_anchor`]).
    #[must_use]
    pub fn anchor_name_for(x: u32, y: u32, w: u32, h: u32) -> String {
        // Exact midpoint exists only for odd dimensions (x*2+1 == w ⇔ x == w/2, w odd).
        let mid_x = x.saturating_mul(2).saturating_add(1) == w;
        let mid_y = y.saturating_mul(2).saturating_add(1) == h;
        if y == 0 && mid_x {
            "n".to_owned()
        } else if y + 1 == h && mid_x {
            "s".to_owned()
        } else if x == 0 && mid_y {
            "w".to_owned()
        } else if x + 1 == w && mid_y {
            "e".to_owned()
        } else {
            format!("{x}_{y}")
        }
    }

    /// AC6 — re-parses the art text box into the grid: printable ASCII only, at
    /// least one row, at least one non-space cell. On success the rows are replaced
    /// (ragged lines right-padded to a rectangle) and anchors that fall outside the
    /// new extent are dropped; on failure [`rejection`](Self::rejection) is set and
    /// the previous art is preserved.
    pub fn set_art(&mut self, text: &str) -> bool {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return self.reject("the art must have at least one row");
        }
        if lines
            .iter()
            .any(|l| l.chars().any(|c| !is_valid_cell_char(c)))
        {
            return self
                .reject("the art holds a character outside the svgbob alphabet (printable ASCII)");
        }
        if !lines.iter().any(|l| l.chars().any(|c| c != ' ')) {
            return self.reject("the art must contain at least one non-space cell");
        }
        let width = lines
            .iter()
            .map(|l| l.chars().count() as u32)
            .max()
            .unwrap_or(0);
        let rows: Vec<String> = lines
            .iter()
            .map(|line| {
                let mut s = line.to_string();
                for _ in 0..width.saturating_sub(s.chars().count() as u32) {
                    s.push(' ');
                }
                s
            })
            .collect();
        // The grid moved/shrank: anchors outside the new extent are dropped.
        self.anchors
            .retain(|a| a.position.x < width && a.position.y < lines.len() as u32);
        self.rows = rows;
        self.rejection = None;
        true
    }

    fn reject(&mut self, message: &str) -> bool {
        self.rejection = Some(message.to_owned());
        false
    }

    /// AC6 — resize-toolset validation: when enabled, every edge needs a valid insert
    /// + remove character and each enabled parity pair a complete pair.
    pub fn resize_validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if !self.resize.enabled {
            return errors;
        }
        for (edge, rule) in [
            ("top", &self.resize.top),
            ("bottom", &self.resize.bottom),
            ("left", &self.resize.left),
            ("right", &self.resize.right),
        ] {
            for (field, value) in [("insert", &rule.insert), ("remove", &rule.remove)] {
                if !is_rule_char(value) {
                    errors.push(format!(
                        "{edge} edge: the {field} character must be a single non-whitespace printable ASCII character"
                    ));
                }
            }
            for (parity, enabled, pair) in [
                (
                    "even",
                    rule.even_enabled,
                    (&rule.even_insert, &rule.even_remove),
                ),
                (
                    "odd",
                    rule.odd_enabled,
                    (&rule.odd_insert, &rule.odd_remove),
                ),
            ] {
                if enabled && (!is_rule_char(pair.0) || !is_rule_char(pair.1)) {
                    errors.push(format!(
                        "{edge} edge: the {parity} pair needs both an insert and a remove character"
                    ));
                }
            }
        }
        errors
    }

    /// AC2 — what blocks saving: an invalid name/collection token, or no anchors.
    /// (Grid characters are validated on input; this is the last line of defense.)
    #[must_use]
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if !is_valid_name_token(&self.name) {
            errors.push(
                "the template name must be a single token without '/' or whitespace".to_owned(),
            );
        }
        if !is_valid_name_token(&self.collection) {
            errors.push(
                "the collection name must be a single token without '/' or whitespace".to_owned(),
            );
        }
        if self.anchors.is_empty() {
            errors.push("at least one anchor is required — connectors bind to anchors".to_owned());
        }
        for row in &self.rows {
            if row.chars().any(|c| !is_valid_cell_char(c)) {
                errors.push("the grid holds a character outside the svgbob alphabet".to_owned());
                break;
            }
        }
        // AC6 — an enabled resize toolset must be complete and valid before saving.
        errors.extend(self.resize_validation_errors());
        errors
    }

    /// The seam-level payload for [`ShapeCatalog::register`](ui_api::ShapeCatalog::register).
    /// An enabled, valid resize toolset becomes the template's `resize_rules` (S-024
    /// AC6); a disabled one stays `None` (the shape remains fixed-size, FR12).
    #[must_use]
    pub fn to_new_template(&self) -> NewTemplate {
        let resize_rules = if self.resize.enabled && self.resize_validation_errors().is_empty() {
            Some(ResizeRuleSpec {
                top: edge_spec(&self.resize.top),
                bottom: edge_spec(&self.resize.bottom),
                left: edge_spec(&self.resize.left),
                right: edge_spec(&self.resize.right),
            })
        } else {
            None
        };
        NewTemplate {
            name: self.name.clone(),
            rows: self.rows.clone(),
            anchors: self.anchors.clone(),
            resize_rules,
        }
    }
}

/// One edge of the resize toolset (S-024 AC6) — single-character fields, parity pairs
/// revealed by their enable toggles.
#[derive(Debug, Clone, Default)]
pub struct EdgeRuleDraft {
    /// The character inserted when the shape grows on this edge.
    pub insert: String,
    /// The character removed when it shrinks.
    pub remove: String,
    /// Whether the even-parity override pair is in use.
    pub even_enabled: bool,
    /// Whether the odd-parity override pair is in use.
    pub odd_enabled: bool,
    /// Even-parity insert character (validated only when `even_enabled`).
    pub even_insert: String,
    /// Even-parity remove character.
    pub even_remove: String,
    /// Odd-parity insert character (validated only when `odd_enabled`).
    pub odd_insert: String,
    /// Odd-parity remove character.
    pub odd_remove: String,
}

/// The resize toolset for one template (S-024 AC6): off by default, and when on all
/// four edges are required — mirroring `ResizeRuleSpec`'s completeness.
#[derive(Debug, Clone, Default)]
pub struct ResizeRulesDraft {
    /// Master toggle — the toolset is inert (and unvalidated) while off.
    pub enabled: bool,
    /// The top edge's rule.
    pub top: EdgeRuleDraft,
    /// The bottom edge's rule.
    pub bottom: EdgeRuleDraft,
    /// The left edge's rule.
    pub left: EdgeRuleDraft,
    /// The right edge's rule.
    pub right: EdgeRuleDraft,
}

/// Maps a validated draft edge to its seam-level spec (S-024 AC6).
fn edge_spec(e: &EdgeRuleDraft) -> EdgeRuleSpec {
    EdgeRuleSpec {
        insert: e.insert.chars().next().unwrap_or(' '),
        remove: e.remove.chars().next().unwrap_or(' '),
        even: if e.even_enabled {
            Some(ParityCharsSpec {
                insert: e.even_insert.chars().next().unwrap_or(' '),
                remove: e.even_remove.chars().next().unwrap_or(' '),
            })
        } else {
            None
        },
        odd: if e.odd_enabled {
            Some(ParityCharsSpec {
                insert: e.odd_insert.chars().next().unwrap_or(' '),
                remove: e.odd_remove.chars().next().unwrap_or(' '),
            })
        } else {
            None
        },
    }
}

/// A valid resize-rule character: exactly one printable ASCII, non-whitespace.
fn is_rule_char(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(c), None) if matches!(c as u32, 0x21..=0x7e)
    )
}

/// The inverse of [`edge_spec`] — restores a seam spec into editable draft fields.
fn edge_draft(spec: &EdgeRuleSpec) -> EdgeRuleDraft {
    EdgeRuleDraft {
        insert: spec.insert.to_string(),
        remove: spec.remove.to_string(),
        even_enabled: spec.even.is_some(),
        odd_enabled: spec.odd.is_some(),
        even_insert: spec.even.map(|p| p.insert.to_string()).unwrap_or_default(),
        even_remove: spec.even.map(|p| p.remove.to_string()).unwrap_or_default(),
        odd_insert: spec.odd.map(|p| p.insert.to_string()).unwrap_or_default(),
        odd_remove: spec.odd.map(|p| p.remove.to_string()).unwrap_or_default(),
    }
}

/// An open editor session: the draft plus transient UI state. No egui types here —
/// the window drawing lives in `app.rs`, everything decidable is unit-tested on this
/// struct headlessly.
#[derive(Debug, Clone)]
pub struct TemplateEditorSession {
    pub draft: TemplateDraft,
    /// The cell currently selected for character input.
    pub selected_cell: Option<(u32, u32)>,
    /// When true, clicking a cell toggles an anchor instead of selecting it.
    pub anchor_mode: bool,
    /// The single-character input bound to the selected cell (committed on change).
    pub cell_input: String,
    /// The art text box's buffer — the primary art input (S-024 AC6). Re-parsed into
    /// [`TemplateDraft::rows`] on every change; reverts to the last accepted art when
    /// rejected.
    pub art_text: String,
    /// The size the resize preview is rendered at (S-024 AC6) — defaults to the
    /// template's natural extent and follows it until changed.
    pub preview_width: u32,
    pub preview_height: u32,
}

impl TemplateEditorSession {
    /// AC6 — commits the art text box into the draft. Returns `true` when accepted;
    /// on rejection the buffer is reverted to the last accepted art (the previous art
    /// is preserved) and the inline hint is set.
    pub fn apply_art(&mut self) -> bool {
        if self.draft.set_art(&self.art_text) {
            true
        } else {
            self.art_text = self.draft.rows.join("\n");
            false
        }
    }

    /// Re-syncs the art buffer from the grid after cell painting (grid → text).
    pub fn sync_art(&mut self) {
        self.art_text = self.draft.rows.join("\n");
        // The preview target follows a changed natural extent.
        self.preview_width = self.draft.width();
        self.preview_height = self.draft.height();
    }

    /// AC6 — paints one cell (alphabet-validated) and re-syncs the art buffer. Returns
    /// whether the input was accepted.
    pub fn paint_cell(&mut self, x: u32, y: u32, c: char) -> bool {
        let ok = self.draft.set_cell(x, y, c);
        if ok {
            self.sync_art();
        }
        ok
    }

    /// AC6 — clears one cell and re-syncs the art buffer.
    pub fn clear_cell(&mut self, x: u32, y: u32) {
        self.draft.clear_cell(x, y);
        self.sync_art();
    }

    /// AC6 — the resize rule set as a seam spec, for the live preview: `None` when the
    /// toolset is disabled or incomplete (the preview then shows the natural size).
    #[must_use]
    pub fn resize_spec(&self) -> Option<ResizeRuleSpec> {
        if self.draft.resize.enabled && self.draft.resize_validation_errors().is_empty() {
            Some(ResizeRuleSpec {
                top: edge_spec(&self.draft.resize.top),
                bottom: edge_spec(&self.draft.resize.bottom),
                left: edge_spec(&self.draft.resize.left),
                right: edge_spec(&self.draft.resize.right),
            })
        } else {
            None
        }
    }
}

impl TemplateEditorSession {
    /// A fresh blank canvas. Fixed 12×8 grid: the template is saved exactly as drawn
    /// (no auto-trim — empty margins are part of the template), so draw from the
    /// top-left corner.
    #[must_use]
    pub fn new_blank(collection: impl Into<String>, name: impl Into<String>) -> Self {
        let draft = TemplateDraft::new(collection, name, 12, 8);
        Self {
            art_text: draft.rows.join("\n"),
            preview_width: draft.width(),
            preview_height: draft.height(),
            draft,
            selected_cell: None,
            anchor_mode: false,
            cell_input: String::new(),
        }
    }

    /// Editing an existing template (S-017 AC3) — same flow as creating one.
    #[must_use]
    pub fn from_existing(info: &ShapeInfo, anchors: Vec<AnchorInfo>) -> Self {
        let mut draft = TemplateDraft::from_existing(info, anchors);
        // S-024: restore the declared rules so editing and saving a rule-bearing
        // template does not silently drop them.
        if let Some(spec) = &info.resize_rules {
            draft.resize.enabled = true;
            draft.resize.top = edge_draft(&spec.top);
            draft.resize.bottom = edge_draft(&spec.bottom);
            draft.resize.left = edge_draft(&spec.left);
            draft.resize.right = edge_draft(&spec.right);
        }
        Self {
            art_text: draft.rows.join("\n"),
            preview_width: draft.width(),
            preview_height: draft.height(),
            draft,
            selected_cell: None,
            anchor_mode: false,
            cell_input: String::new(),
        }
    }
}

/// A single non-empty token without `/` — the name grammar of `collection/name` ids (C8).
fn is_valid_name_token(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.chars().any(|c| c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::geometry::CellSize;

    fn draft() -> TemplateDraft {
        TemplateDraft::new("stars", "star", 9, 5)
    }

    /// AC1 — svgbob-alphabet input is accepted into the exact cell.
    #[test]
    fn set_cell_accepts_alphabet_chars() {
        let mut d = draft();
        assert!(d.set_cell(2, 3, '#'));
        assert_eq!(&d.rows[3][2..3], "#");
        // Space is the empty cell and part of the alphabet.
        assert!(d.set_cell(4, 0, ' '));
        // The rejection hint clears on the next accepted edit.
        d.rejection = Some("old".to_owned());
        assert!(d.set_cell(0, 0, '+'));
        assert_eq!(d.rejection, None);
    }

    /// AC1 — non-alphabet input is rejected with an inline hint, cell unchanged.
    #[test]
    fn set_cell_rejects_non_alphabet_with_inline_hint() {
        let mut d = draft();
        assert!(!d.set_cell(0, 0, 'é'));
        assert!(
            d.rejection
                .as_deref()
                .unwrap_or("")
                .contains("svgbob alphabet")
        );
        // And CJK / control characters too.
        assert!(!d.set_cell(1, 1, '中'));
        assert!(!d.set_cell(2, 2, '\u{1b}'));
        assert_eq!(d.rows[0], "         ");
    }

    /// Clearing a cell restores the empty-cell space.
    #[test]
    fn clear_cell_resets_to_space() {
        let mut d = draft();
        d.set_cell(3, 2, 'X');
        d.clear_cell(3, 2);
        assert_eq!(d.rows[2].chars().nth(3), Some(' '));
    }

    /// Out-of-bounds edits are no-ops, not panics.
    #[test]
    fn out_of_bounds_edits_are_noops() {
        let mut d = draft();
        assert!(!d.set_cell(99, 0, '#'));
        assert!(!d.set_cell(0, 99, '#'));
        d.clear_cell(99, 99);
        assert_eq!(d.rows.len(), 5);
    }

    /// AC2 — anchor toggling: edge midpoints get the standard names, other cells a
    /// position token; toggling again removes.
    #[test]
    fn toggle_anchor_uses_standard_edge_names_and_removes() {
        let mut d = draft(); // 9×5
        assert_eq!(d.toggle_anchor(4, 0).as_deref(), Some("n"));
        assert_eq!(d.toggle_anchor(4, 4).as_deref(), Some("s"));
        assert_eq!(d.toggle_anchor(0, 2).as_deref(), Some("w"));
        assert_eq!(d.toggle_anchor(8, 2).as_deref(), Some("e"));
        // A non-midpoint cell gets the position token.
        assert_eq!(d.toggle_anchor(3, 1).as_deref(), Some("3_1"));

        // Toggling an anchored cell removes it again.
        d.toggle_anchor(4, 0);
        assert_eq!(d.anchors.len(), 4);
        assert!(!d.anchors.iter().any(|a| a.name == "n"));
    }

    /// Even dimensions have no exact midpoint — position tokens there too.
    #[test]
    fn even_grids_have_no_standard_names() {
        let mut d = TemplateDraft::new("c", "t", 4, 2);
        assert_eq!(d.toggle_anchor(1, 0).as_deref(), Some("1_0"));
        assert_eq!(d.toggle_anchor(3, 0).as_deref(), Some("3_0"));
    }

    /// AC2 — saving is blocked without anchors and with invalid name tokens.
    #[test]
    fn validation_requires_a_name_and_one_anchor() {
        let mut d = draft();
        assert!(d.validation_errors().iter().any(|e| e.contains("anchor")));

        d.toggle_anchor(4, 0);
        assert!(
            d.validation_errors().is_empty(),
            "valid now: {:?}",
            d.validation_errors()
        );

        d.name = "bad name".to_owned();
        assert!(d.validation_errors().iter().any(|e| e.contains("name")));
        d.name = "ok/name".to_owned();
        assert!(d.validation_errors().iter().any(|e| e.contains("name")));
    }

    /// S-024 — editing an existing rule-bearing template restores its rules into
    /// the resize toolset, so saving it back does not silently drop them.
    #[test]
    fn from_existing_restores_declared_resize_rules() {
        let spec = ResizeRuleSpec {
            top: EdgeRuleSpec {
                insert: '-',
                remove: '-',
                even: Some(ParityCharsSpec {
                    insert: '=',
                    remove: '=',
                }),
                odd: None,
            },
            bottom: EdgeRuleSpec {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            left: EdgeRuleSpec {
                insert: '|',
                remove: '|',
                even: None,
                odd: Some(ParityCharsSpec {
                    insert: '(',
                    remove: '(',
                }),
            },
            right: EdgeRuleSpec {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
        };
        let info = ShapeInfo {
            type_id: "stars/star".to_owned(),
            name: "star".to_owned(),
            size: diagram_model::geometry::CellSize::new(3, 2).expect("valid"),
            collection: Some("stars".to_owned()),
            preview: vec!["#.#".to_owned()],
            resizable: true,
            min_size: CellSize::new(1, 1).expect("valid"),
            resize_rules: Some(spec.clone()),
        };
        let session = TemplateEditorSession::from_existing(&info, Vec::new());
        assert!(session.draft.resize.enabled);
        // The restored draft round-trips through the seam spec unchanged.
        assert_eq!(session.resize_spec().as_ref(), Some(&spec));
    }

    /// The seam payload carries the draft across unchanged.
    #[test]
    fn to_new_template_carries_the_draft() {
        let mut d = draft();
        d.set_cell(0, 0, '#');
        d.toggle_anchor(4, 0);
        let nt = d.to_new_template();
        assert_eq!(nt.name, "star");
        assert_eq!(nt.rows.len(), 5);
        assert_eq!(nt.rows[0].chars().next(), Some('#'));
        assert_eq!(nt.anchors.len(), 1);
        assert_eq!(nt.anchors[0].name, "n");
    }

    /// Editing an existing template loads its grid (padded) and anchors.
    #[test]
    fn from_existing_loads_grid_and_anchors() {
        let info = ShapeInfo {
            type_id: "stars/star".to_owned(),
            name: "star".to_owned(),
            size: diagram_model::geometry::CellSize::new(3, 2).expect("valid"),
            collection: Some("stars".to_owned()),
            // Ragged preview (short rows) — padded to the shape's width.
            preview: vec!["#.#".to_owned(), "##".to_owned()],
            resizable: false,
            min_size: CellSize::new(1, 1).expect("valid"),
            resize_rules: None,
        };
        let anchors = vec![AnchorInfo {
            name: "n".to_owned(),
            position: CellPos::new(1, 0),
        }];
        let d = TemplateDraft::from_existing(&info, anchors.clone());
        assert_eq!(d.collection, "stars");
        assert_eq!(d.rows, vec!["#.#".to_owned(), "## ".to_owned()]);
        assert_eq!(d.anchors, anchors);
    }

    // ── S-024 AC6: the art text box is the primary input ──

    /// AC6 — typing/pasting re-parses into rows; ragged lines are padded to a
    /// rectangle (the input's own max width, not the previous draft's).
    #[test]
    fn set_art_reparses_into_padded_rows() {
        let mut d = draft(); // 9×5
        assert!(d.set_art("+-+\n| |\n+-"));
        assert_eq!(
            d.rows,
            vec!["+-+".to_owned(), "| |".to_owned(), "+- ".to_owned()]
        );
        assert_eq!(d.width(), 3);
        assert_eq!(d.height(), 3);
    }

    /// AC6 — non-alphabet input is rejected with a hint; the previous art is preserved.
    #[test]
    fn set_art_rejects_non_alphabet_and_keeps_previous() {
        let mut d = draft();
        assert!(d.set_art("##\n##"));
        assert!(!d.set_art("##\n#é"));
        assert!(
            d.rejection
                .as_deref()
                .unwrap_or("")
                .contains("svgbob alphabet")
        );
        // Previous art preserved — rows untouched.
        assert_eq!(d.rows, vec!["##".to_owned(), "##".to_owned()]);
    }

    /// AC6 — empty (all-space) and zero-row input are rejected.
    #[test]
    fn set_art_rejects_blank_and_empty() {
        let mut d = draft();
        assert!(!d.set_art("   \n  "));
        assert!(d.rejection.as_deref().unwrap_or("").contains("non-space"));
        assert!(!d.set_art(""));
        assert!(
            d.rejection
                .as_deref()
                .unwrap_or("")
                .contains("at least one row")
        );
    }

    /// AC6 — shrinking the art drops anchors that fall outside the new extent.
    #[test]
    fn set_art_drops_out_of_extent_anchors() {
        let mut d = draft(); // 9×5
        assert!(d.set_art("#########\n#       #\n#########"));
        d.toggle_anchor(8, 1); // right edge of the 9-wide art
        assert_eq!(d.anchors.len(), 1);
        assert!(d.set_art("###\n###"));
        assert!(
            d.anchors.is_empty(),
            "anchor at x=8 is outside the 3-wide grid"
        );
    }

    /// AC6 — the session commits the buffer, reverting it on rejection.
    #[test]
    fn apply_art_reverts_the_buffer_on_rejection() {
        let mut s = TemplateEditorSession::new_blank("stars", "star");
        s.art_text = "##\n##".to_owned();
        assert!(s.apply_art());
        assert_eq!(s.draft.rows.len(), 2);

        s.art_text = "##\n#中".to_owned();
        assert!(!s.apply_art());
        // The buffer is back on the last accepted art.
        assert_eq!(s.art_text, "##\n##");
        assert!(s.draft.rejection.is_some());
    }

    // ── S-024 AC6: the resize toolset ──

    /// AC6 — a disabled toolset adds no validation errors and no rules.
    #[test]
    fn resize_toolset_disabled_is_inert() {
        let mut d = draft();
        assert!(d.resize_validation_errors().is_empty());
        assert_eq!(d.to_new_template().resize_rules, None);

        // Enabled but empty → all four edges demanded.
        d.resize.enabled = true;
        let errs = d.resize_validation_errors();
        assert_eq!(errs.len(), 8, "4 edges × (insert + remove): {errs:?}");
    }

    /// AC6 — a complete toolset passes validation and lands in the seam payload.
    #[test]
    fn resize_toolset_complete_lands_in_seam_payload() {
        let mut d = draft();
        d.resize.enabled = true;
        for edge in [
            &mut d.resize.top,
            &mut d.resize.bottom,
            &mut d.resize.left,
            &mut d.resize.right,
        ] {
            edge.insert = "+".to_owned();
            edge.remove = "+".to_owned();
        }
        assert!(d.resize_validation_errors().is_empty());
        assert!(d.validation_errors().iter().any(|e| e.contains("anchor")));
        d.toggle_anchor(4, 0);
        assert!(
            d.validation_errors().is_empty(),
            "{:?}",
            d.validation_errors()
        );

        let nt = d.to_new_template();
        let spec = nt.resize_rules.expect("rules present");
        assert_eq!(spec.top.insert, '+');
        assert_eq!(spec.right.remove, '+');
        assert_eq!(spec.bottom.even, None);
    }

    /// AC6 — parity pairs are validated only when enabled; whitespace chars rejected.
    #[test]
    fn resize_parity_pairs_validated_when_enabled() {
        let mut d = draft();
        d.resize.enabled = true;
        for edge in [
            &mut d.resize.top,
            &mut d.resize.bottom,
            &mut d.resize.left,
            &mut d.resize.right,
        ] {
            edge.insert = "+".to_owned();
            edge.remove = "+".to_owned();
        }
        assert!(d.resize_validation_errors().is_empty());

        // Enable the even pair on one edge with only an insert char → error.
        d.resize.top.even_enabled = true;
        d.resize.top.even_insert = ".".to_owned();
        let errs = d.resize_validation_errors();
        assert!(errs.len() == 1 && errs[0].contains("even pair"), "{errs:?}");

        // Complete the pair → valid; a multi-char or whitespace char is rejected.
        d.resize.top.even_remove = ".".to_owned();
        assert!(d.resize_validation_errors().is_empty());
        d.resize.top.even_insert = "..".to_owned();
        assert!(
            d.resize_validation_errors()
                .iter()
                .any(|e| e.contains("top edge"))
        );
        d.resize.top.even_insert = " ".to_owned();
        assert!(!is_rule_char(" "), "whitespace is not a rule character");

        // The completed even pair lands in the spec.
        d.resize.top.even_insert = ".".to_owned();
        let spec = d.to_new_template().resize_rules.expect("rules present");
        assert_eq!(
            spec.top.even,
            Some(ParityCharsSpec {
                insert: '.',
                remove: '.'
            })
        );
        assert_eq!(spec.left.odd, None);
    }

    // ── Cell painting re-syncs the art buffer (grid → text) ──

    /// AC6 — painting a cell updates the grid and the art buffer together.
    #[test]
    fn paint_cell_resyncs_the_art_buffer() {
        let mut s = TemplateEditorSession::new_blank("stars", "star");
        assert!(s.paint_cell(0, 0, '#'));
        assert_eq!(s.art_text.chars().next(), Some('#'));
        // Rejected input: buffer untouched.
        let before = s.art_text.clone();
        assert!(!s.paint_cell(1, 1, 'é'));
        assert_eq!(s.art_text, before);
    }

    /// AC6 — clearing a cell re-syncs the art buffer back to spaces.
    #[test]
    fn clear_cell_resyncs_the_art_buffer() {
        let mut s = TemplateEditorSession::new_blank("stars", "star");
        s.paint_cell(0, 0, '#');
        s.clear_cell(0, 0);
        assert_eq!(s.art_text.chars().next(), Some(' '));
    }

    /// AC6 — the session exposes the spec for the live preview only when complete.
    #[test]
    fn resize_spec_is_none_until_complete() {
        let mut s = TemplateEditorSession::new_blank("stars", "star");
        assert_eq!(s.resize_spec(), None);
        s.draft.resize.enabled = true;
        for edge in [
            &mut s.draft.resize.top,
            &mut s.draft.resize.bottom,
            &mut s.draft.resize.left,
            &mut s.draft.resize.right,
        ] {
            edge.insert = "+".to_owned();
            edge.remove = "+".to_owned();
        }
        assert!(s.resize_spec().is_some());
    }
}
