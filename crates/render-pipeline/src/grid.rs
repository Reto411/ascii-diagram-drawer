//! The [`ResolvedGrid`] value (§8.4.3) and scale expansion (FR8).

use crate::is_line_glyph;

/// A resolved character grid: the single derived intermediate that all outputs
/// (text/SVG/PNG) render from ([render pipeline](../../../doc/arc42/12-concepts/render-pipeline.md)).
///
/// Cells are stored row-major; `' '` marks an unoccupied cell. The rendered extent is
/// exactly the bounding box of occupied cells — there is no canvas to size it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGrid {
    width: u32,
    height: u32,
    cells: Vec<char>,
}

impl ResolvedGrid {
    /// Crate-internal constructor from an already-resolved row-major cell buffer.
    #[must_use]
    pub(crate) fn new(width: u32, height: u32, cells: Vec<char>) -> Self {
        debug_assert_eq!(cells.len(), (width as usize) * (height as usize));
        Self {
            width,
            height,
            cells,
        }
    }

    /// The empty grid (0×0) — e.g. a diagram with no objects.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            cells: Vec::new(),
        }
    }

    /// Builds a grid from equal-width rows of printable ASCII (test/golden helper).
    ///
    /// An empty slice yields the empty grid. Ragged rows and non-printable-ASCII
    /// characters are rejected — resolved grids hold exactly one glyph per cell.
    pub fn from_rows(rows: &[&str]) -> Result<Self, GridError> {
        if rows.is_empty() {
            return Ok(Self::empty());
        }
        let width = rows[0].chars().count() as u32;
        let mut cells = Vec::with_capacity(rows.len() * width as usize);
        for (y, row) in rows.iter().enumerate() {
            let found = row.chars().count() as u32;
            if found != width {
                return Err(GridError::RaggedRow {
                    row: y as u32,
                    expected: width,
                    found,
                });
            }
            for (x, c) in row.chars().enumerate() {
                let cp = c as u32;
                if !(0x20..=0x7E).contains(&cp) {
                    return Err(GridError::InvalidChar {
                        row: y as u32,
                        column: x as u32,
                        char: c,
                    });
                }
                cells.push(c);
            }
        }
        Ok(Self {
            width,
            height: rows.len() as u32,
            cells,
        })
    }

    /// The grid width in cells (0 for the empty grid).
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// The grid height in cells (0 for the empty grid).
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Whether the grid is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The glyph at `(x, y)`, or `None` out of bounds.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<char> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.cells[(y as usize) * self.width as usize + x as usize])
    }

    /// A full row as a string, or `None` out of bounds.
    #[must_use]
    pub fn row(&self, y: u32) -> Option<String> {
        if y >= self.height {
            return None;
        }
        let start = (y as usize) * self.width as usize;
        Some(
            self.cells[start..start + self.width as usize]
                .iter()
                .collect(),
        )
    }

    /// All rows joined by `'\n'` (`""` for the empty grid).
    #[must_use]
    pub fn to_text(&self) -> String {
        (0..self.height)
            .map(|y| self.row(y).expect("row in bounds"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Errors from [`ResolvedGrid::from_rows`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// A row is not the same width as the first row.
    RaggedRow {
        /// 0-based row index.
        row: u32,
        /// The width (in cells) of the first row.
        expected: u32,
        /// The width (in cells) of the offending row.
        found: u32,
    },
    /// A cell holds a non-printable-ASCII character.
    InvalidChar {
        /// 0-based row index.
        row: u32,
        /// 0-based column index.
        column: u32,
        /// The offending character.
        char: char,
    },
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RaggedRow {
                row,
                expected,
                found,
            } => write!(
                f,
                "row {row} has width {found}, expected {expected} (all rows must be equal width)"
            ),
            Self::InvalidChar { row, column, char } => write!(
                f,
                "cell ({column}, {row}) holds non-printable-ASCII character {char:?}"
            ),
        }
    }
}

impl std::error::Error for GridError {}

/// Expands a resolved grid by an integer scale factor (FR8): each source cell becomes
/// an `scale × scale` block.
///
/// - **Line glyphs** (svgbob-alphabet characters) fill their whole block, so edges and
///   junctions stay continuous lines at every scale.
/// - **Content glyphs** (letters, digits, punctuation — labels and free text) are placed
///   at the defined sub-cell: centered in the block, biased top-left for even scales.
/// - `' '` stays `' '`.
///
/// Scale 1 is the identity; scale < 1 is treated as 1 (the model guarantees ≥ 1).
#[must_use]
pub fn expand(grid: &ResolvedGrid, scale: u32) -> ResolvedGrid {
    let s = scale.max(1);
    if s == 1 || grid.is_empty() {
        return grid.clone();
    }
    let sub = (s - 1) / 2; // content sub-cell offset (centered, top-left biased)
    let width = grid.width().saturating_mul(s);
    let height = grid.height().saturating_mul(s);
    let mut cells = vec![' '; (width as usize) * (height as usize)];
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let c = grid.get(x, y).expect("cell in bounds");
            if c == ' ' {
                continue;
            }
            let fill = is_line_glyph(c);
            for dy in 0..s {
                for dx in 0..s {
                    if fill || (dx == sub && dy == sub) {
                        cells[(y * s + dy) as usize * width as usize + (x * s + dx) as usize] = c;
                    }
                }
            }
        }
    }
    ResolvedGrid {
        width,
        height,
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::{GridError, ResolvedGrid, expand};

    fn grid(rows: &[&str]) -> ResolvedGrid {
        ResolvedGrid::from_rows(rows).expect("test grids are valid")
    }

    #[test]
    fn from_rows_round_trips() {
        let g = grid(&["-+-", "| |"]);
        assert_eq!(g.width(), 3);
        assert_eq!(g.height(), 2);
        assert_eq!(g.to_text(), "-+-\n| |");
        assert_eq!(g.get(1, 0), Some('+'));
        assert_eq!(g.get(3, 0), None);
        assert_eq!(g.row(1), Some("| |".to_owned()));
    }

    #[test]
    fn from_rows_rejects_ragged_and_invalid() {
        let err = ResolvedGrid::from_rows(&["ab", "abc"]).unwrap_err();
        assert_eq!(
            err,
            GridError::RaggedRow {
                row: 1,
                expected: 2,
                found: 3
            }
        );
        let err = ResolvedGrid::from_rows(&["a\x00b"]).unwrap_err();
        assert_eq!(
            err,
            GridError::InvalidChar {
                row: 0,
                column: 1,
                char: '\x00'
            }
        );
    }

    #[test]
    fn empty_grid() {
        let g = ResolvedGrid::empty();
        assert!(g.is_empty());
        assert_eq!(g.width(), 0);
        assert_eq!(g.height(), 0);
        assert_eq!(g.to_text(), "");
        assert_eq!(ResolvedGrid::from_rows(&[]), Ok(g.clone()));
    }

    #[test]
    fn scale_one_is_identity() {
        let g = grid(&["-+-", "| |"]);
        assert_eq!(expand(&g, 1), g);
    }

    #[test]
    fn scale_zero_is_treated_as_one() {
        let g = grid(&["a"]);
        assert_eq!(expand(&g, 0), g);
    }

    #[test]
    fn scale_two_fills_line_glyphs_and_places_content_subcell() {
        // `+` (line glyph) fills its 2x2 block; `A` (content) lands at sub-cell (0,0).
        let out = expand(&grid(&["+A"]), 2);
        assert_eq!(out.width(), 4);
        assert_eq!(out.height(), 2);
        // Row 0: ++ (filled `+` block) then A at sub-cell (0,0) of its block.
        assert_eq!(out.to_text(), "++A \n++  ");
    }

    #[test]
    fn scale_three_centers_content_subcell() {
        let out = expand(&grid(&["A"]), 3);
        assert_eq!(out.to_text(), "   \n A \n   ");
    }

    #[test]
    fn scale_two_keeps_edges_continuous() {
        // A horizontal line stays a continuous (2-row-thick) line at scale 2.
        let out = expand(&grid(&["---"]), 2);
        assert_eq!(out.to_text(), "------\n------");
    }

    #[test]
    fn scale_expands_empty_grid() {
        assert_eq!(expand(&ResolvedGrid::empty(), 3), ResolvedGrid::empty());
    }
}
