//! Core catalog value types (ADR 007): [`TemplateGrid`], [`AnchorSet`], [`ShapeType`],
//! [`Collection`] — and the placeholder [`ResizeRules`] that v1 leaves empty.

use std::collections::BTreeMap;

use crate::error::{AnchorSetError, GridError};
use crate::geometry::{CellPos, CellSize};

/// A fixed-size rectangular grid of glyph characters for a shape template
/// ([shape-template](../../../doc/arc42/12-concepts/shape-template.md)).
///
/// Every cell holds exactly one printable ASCII character U+0020–U+007E; space is the
/// empty cell. Drawing characters are interpreted by the renderer; all other cells are
/// stamped verbatim as an opaque stamp (C6). A template's size is exactly its grid —
/// there is no separate size field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateGrid {
    /// Width in cells (≥ 1).
    width: u32,
    /// Height in cells (≥ 1).
    height: u32,
    /// Cells row-major: `cells[row * width + column]`.
    cells: Vec<char>,
}

impl TemplateGrid {
    /// Builds a grid from non-empty, equal-length rows of printable ASCII.
    ///
    /// # Errors
    /// [`GridError`] when the rows are empty, ragged, or contain a character outside
    /// U+0020–U+007E.
    pub fn from_rows(rows: &[&str]) -> Result<Self, GridError> {
        let width = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .filter(|w| *w > 0);
        let Some(width) = width else {
            return Err(GridError::Empty);
        };
        if rows.iter().any(|row| row.chars().count() != width) {
            return Err(GridError::RaggedRows);
        }
        let mut cells = Vec::with_capacity(rows.len() * width);
        for (row, line) in rows.iter().enumerate() {
            for (column, c) in line.chars().enumerate() {
                if !matches!(c as u32, 0x20..=0x7e) {
                    return Err(GridError::InvalidChar {
                        row,
                        column,
                        char: c,
                    });
                }
                cells.push(c);
            }
        }
        Ok(Self {
            width: width as u32,
            height: rows.len() as u32,
            cells,
        })
    }

    /// The grid's width in cells.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The grid's height in cells.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The character at `pos`, or `None` when the position is outside the grid.
    #[must_use]
    pub fn get(&self, pos: CellPos) -> Option<char> {
        if pos.x >= self.width || pos.y >= self.height {
            return None;
        }
        self.cells
            .get((pos.y * self.width + pos.x) as usize)
            .copied()
    }

    /// The full row at `y` (exactly `width` chars), or `None` when out of range.
    #[must_use]
    pub fn row(&self, y: u32) -> Option<String> {
        if y >= self.height {
            return None;
        }
        let start = (y * self.width) as usize;
        Some(
            self.cells[start..start + self.width as usize]
                .iter()
                .collect(),
        )
    }

    /// The grid's dimensions as a [`CellSize`].
    #[must_use]
    pub fn size(&self) -> CellSize {
        CellSize::new(self.width, self.height)
    }
}

/// Named anchor points of a shape type — the cells where connectors may bind (C7).
///
/// Anchor names are unique within the set; iteration is deterministic (alphabetical).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnchorSet {
    anchors: BTreeMap<String, CellPos>,
}

impl AnchorSet {
    /// An empty anchor set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares an anchor at `pos`.
    ///
    /// # Errors
    /// [`AnchorSetError::DuplicateName`] when the name is already declared in this set.
    pub fn add(&mut self, name: &str, pos: CellPos) -> Result<(), AnchorSetError> {
        if self.anchors.contains_key(name) {
            return Err(AnchorSetError::DuplicateName(name.to_owned()));
        }
        self.anchors.insert(name.to_owned(), pos);
        Ok(())
    }

    /// The position of `name`, or `None` when not declared.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<CellPos> {
        self.anchors.get(name).copied()
    }

    /// The number of declared anchors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Whether no anchors are declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Iterates `(name, position)` pairs in deterministic (alphabetical) order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CellPos)> {
        self.anchors.iter()
    }
}

/// Resize rules for a shape type. v1 has no resize rules — all custom objects are
/// fixed-size; the field is retained on [`ShapeType`] so rules can be added without
/// changing the model (S-005 AC4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResizeRules;

/// How the render pipeline draws a shape type's geometry
/// ([render pipeline](../../../doc/arc42/12-concepts/render-pipeline.md), S-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryKind {
    /// Axis-aligned rectangle: borders are edge connections resolved at the
    /// instance's own `pos`/`size`, so instances of any size join cleanly.
    Rect,
    /// Ellipse fitted to the instance's own `pos`/`size`: the outline is re-rasterized
    /// deterministically at render time (the 9×5 art documents the natural appearance),
    /// and anchors map proportionally like `Rect` (B-001, ADR 013 v3).
    Ellipse,
    /// Fixed glyph pattern stamped opaquely at its natural size (curves cannot be
    /// expressed with N/E/S/W edges; resizing custom templates is a v1+ concern,
    /// ADR 007 point 4).
    Pattern,
}

/// A shape type: a glyph pattern plus its named anchors and (future) resize rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeType {
    /// The catalog key: bare name for built-ins, `collection/name` for custom shapes.
    pub type_id: String,
    /// Display name; unique within a collection (C7).
    pub name: String,
    /// The glyph pattern — the shape's size is exactly this grid.
    pub glyph_pattern: TemplateGrid,
    /// Named anchor points where connectors may bind (C7).
    pub anchor_set: AnchorSet,
    /// How the render pipeline draws this type's geometry (S-006).
    pub geometry: GeometryKind,
    /// Resize rules; `None` in v1 (all custom objects are fixed-size).
    pub resize_rules: Option<ResizeRules>,
}

impl ShapeType {
    /// The shape's size — exactly the glyph grid's dimensions.
    #[must_use]
    pub fn size(&self) -> CellSize {
        self.glyph_pattern.size()
    }
}

/// A named collection of shape types (C7). Identity is case-sensitive; for `.adc`
/// files it comes from the filename stem — there is no in-file name statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    /// The collection name (case-sensitive identity).
    pub name: String,
    /// The declared templates in declaration order.
    pub types: Vec<ShapeType>,
}

impl Collection {
    /// Finds a template by name (case-sensitive), preserving declaration order on ties.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ShapeType> {
        self.types.iter().find(|t| t.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::{AnchorSet, Collection, GeometryKind, ShapeType, TemplateGrid};
    use crate::error::{AnchorSetError, GridError};
    use crate::geometry::{CellPos, CellSize};

    fn grid() -> TemplateGrid {
        TemplateGrid::from_rows(&["abcd", "efgh"]).expect("valid rows")
    }

    /// A valid rectangular grid exposes its dimensions and cells.
    #[test]
    fn template_grid_holds_cells_row_major() {
        let g = grid();
        assert_eq!(g.width(), 4);
        assert_eq!(g.height(), 2);
        assert_eq!(g.get(CellPos::new(0, 0)), Some('a'));
        assert_eq!(g.get(CellPos::new(3, 1)), Some('h'));
        assert_eq!(g.row(0), Some("abcd".to_owned()));
        assert_eq!(g.row(1), Some("efgh".to_owned()));
        assert_eq!(g.size(), CellSize::new(4, 2));
    }

    /// Out-of-range cells and rows are `None`, never panics.
    #[test]
    fn template_grid_lookups_are_bounded() {
        let g = grid();
        assert_eq!(g.get(CellPos::new(4, 0)), None);
        assert_eq!(g.get(CellPos::new(0, 2)), None);
        assert_eq!(g.row(2), None);
    }

    /// Empty and ragged rows are rejected with typed errors.
    #[test]
    fn template_grid_rejects_empty_and_ragged_rows() {
        assert_eq!(TemplateGrid::from_rows(&[]), Err(GridError::Empty));
        assert_eq!(TemplateGrid::from_rows(&[""]), Err(GridError::Empty));
        assert_eq!(
            TemplateGrid::from_rows(&["ab", "cde"]),
            Err(GridError::RaggedRows)
        );
    }

    /// Non-printable-ASCII cells are rejected with their position.
    #[test]
    fn template_grid_rejects_non_printable_cells() {
        assert_eq!(
            TemplateGrid::from_rows(&["a\tb"]),
            Err(GridError::InvalidChar {
                row: 0,
                column: 1,
                char: '\t'
            })
        );
        assert_eq!(
            TemplateGrid::from_rows(&["éx"]),
            Err(GridError::InvalidChar {
                row: 0,
                column: 0,
                char: 'é'
            })
        );
    }

    /// Anchors are unique per name and iterate in deterministic order.
    #[test]
    fn anchor_set_is_unique_and_ordered() {
        let mut set = AnchorSet::new();
        assert!(set.is_empty());
        set.add("s", CellPos::new(4, 3)).expect("fresh name");
        set.add("n", CellPos::new(4, 0)).expect("fresh name");
        assert_eq!(set.len(), 2);
        assert_eq!(set.get("n"), Some(CellPos::new(4, 0)));
        assert_eq!(set.get("w"), None);
        let names: Vec<&String> = set.iter().map(|(name, _)| name).collect();
        assert_eq!(names, vec!["n", "s"]);
    }

    /// Re-adding a declared anchor name is a typed error.
    #[test]
    fn anchor_set_rejects_duplicate_names() {
        let mut set = AnchorSet::new();
        set.add("n", CellPos::new(4, 0)).expect("fresh name");
        assert_eq!(
            set.add("n", CellPos::new(1, 1)),
            Err(AnchorSetError::DuplicateName("n".to_owned()))
        );
        // A rejected duplicate must not move the existing anchor.
        assert_eq!(set.get("n"), Some(CellPos::new(4, 0)));
    }

    /// A shape's size is exactly its glyph grid's dimensions.
    #[test]
    fn shape_type_size_comes_from_the_grid() {
        let shape = ShapeType {
            type_id: "box".to_owned(),
            name: "box".to_owned(),
            glyph_pattern: grid(),
            anchor_set: AnchorSet::new(),
            geometry: GeometryKind::Rect,
            resize_rules: None,
        };
        assert_eq!(shape.size(), CellSize::new(4, 2));
    }

    /// Collection lookup is by exact (case-sensitive) name.
    #[test]
    fn collection_get_is_case_sensitive() {
        let col = Collection {
            name: "mycol".to_owned(),
            types: vec![ShapeType {
                type_id: "mycol/gear".to_owned(),
                name: "gear".to_owned(),
                glyph_pattern: grid(),
                anchor_set: AnchorSet::new(),
                geometry: GeometryKind::Pattern,
                resize_rules: None,
            }],
        };
        assert_eq!(
            col.get("gear").map(|t| t.type_id.as_str()),
            Some("mycol/gear")
        );
        assert_eq!(col.get("Gear"), None);
    }
}
