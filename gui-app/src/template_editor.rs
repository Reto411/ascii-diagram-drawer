//! Template editor state (S-017 AC1/AC2) — pure data + operations, unit-tested
//! headlessly. The egui drawing in `app.rs` stays thin; every user-visible decision
//! (which input is rejected, which anchor name a cell gets, what blocks saving) lives
//! here.
//!
//! The alphabet is svgbob's input set: printable ASCII U+0020–U+007E, where space is
//! the empty cell (ADR 001). Anything else is rejected with an inline hint (AC1).

use diagram_model::geometry::CellPos;
use ui_api::{AnchorInfo, NewTemplate, ShapeInfo};

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
        errors
    }

    /// The seam-level payload for [`ShapeCatalog::register`](ui_api::ShapeCatalog::register).
    #[must_use]
    pub fn to_new_template(&self) -> NewTemplate {
        NewTemplate {
            name: self.name.clone(),
            rows: self.rows.clone(),
            anchors: self.anchors.clone(),
        }
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
}

impl TemplateEditorSession {
    /// A fresh blank canvas. Fixed 12×8 grid: the template is saved exactly as drawn
    /// (no auto-trim — empty margins are part of the template), so draw from the
    /// top-left corner.
    #[must_use]
    pub fn new_blank(collection: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            draft: TemplateDraft::new(collection, name, 12, 8),
            selected_cell: None,
            anchor_mode: false,
            cell_input: String::new(),
        }
    }

    /// Editing an existing template (S-017 AC3) — same flow as creating one.
    #[must_use]
    pub fn from_existing(info: &ShapeInfo, anchors: Vec<AnchorInfo>) -> Self {
        Self {
            draft: TemplateDraft::from_existing(info, anchors),
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
}
