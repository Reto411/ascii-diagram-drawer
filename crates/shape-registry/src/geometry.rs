//! Cell geometry — registry-local value types for the shape catalog.
//!
//! The shape catalog is a leaf context (§8.3 DAG) and owns its own coordinate
//! values: positions are whole cells relative to a shape's top-left origin, the
//! same convention as the core model's cell grid (C5).

/// A position on the cell grid, in whole cells from a shape's top-left origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPos {
    /// Column index (0-based, grows to the right).
    pub x: u32,
    /// Row index (0-based, grows downward).
    pub y: u32,
}

impl CellPos {
    /// Creates a position. Coordinates are unsigned by construction — negative
    /// positions are unrepresentable.
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// A size in whole cells. Instances of a shape type always occupy at least one cell
/// per dimension — glyph grids are constructed with both dimensions ≥ 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellSize {
    /// Width in cells (≥ 1).
    pub width: u32,
    /// Height in cells (≥ 1).
    pub height: u32,
}

impl CellSize {
    /// Creates a size. Callers derive sizes from validated glyph grids, which are
    /// non-zero by construction.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// A rectangular text region inside a template grid (FR19 / CR-016): origin and extent
/// in whole cells relative to the shape's top-left corner. Instances carry per-region
/// content; rendering maps the rect onto the instance extent and stamps content there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region {
    /// Origin column (0-based).
    pub x: u32,
    /// Origin row (0-based).
    pub y: u32,
    /// Width in cells (≥ 1).
    pub width: u32,
    /// Height in cells (≥ 1).
    pub height: u32,
}

impl Region {
    /// Creates a region. Extents of zero are rejected by the parser and the editor;
    /// this constructor does not re-check (callers build regions from validated input).
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether the region fits inside a grid of the given extent (the `.adc` parser's
    /// rule: `x + width ≤ grid_width` and `y + height ≤ grid_height`).
    #[must_use]
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.x.saturating_add(self.width) <= width && self.y.saturating_add(self.height) <= height
    }
}

#[cfg(test)]
mod tests {
    use super::{CellPos, CellSize, Region};

    /// `new` builds the values; equality is by value.
    #[test]
    fn cell_values_are_plain_data() {
        assert_eq!(CellPos::new(3, 5), CellPos { x: 3, y: 5 });
        assert_ne!(CellPos::new(1, 2), CellPos::new(2, 1));
        assert_eq!(
            CellSize::new(4, 2),
            CellSize {
                width: 4,
                height: 2
            }
        );
    }

    /// A region fits when origin + extent stays inside the grid (one-past-the-grid is
    /// out; the last cell index is in).
    #[test]
    fn region_fits_checks_against_the_grid_extent() {
        let r = Region::new(1, 1, 3, 2);
        assert!(r.fits(4, 3)); // 1+3 ≤ 4, 1+2 ≤ 3
        assert!(!r.fits(3, 3)); // 1+3 > 3
        assert!(!r.fits(4, 2)); // 1+2 > 2
        assert!(Region::new(0, 0, 4, 3).fits(4, 3)); // the whole grid
    }
}
