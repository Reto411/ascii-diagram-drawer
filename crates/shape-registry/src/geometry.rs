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

#[cfg(test)]
mod tests {
    use super::{CellPos, CellSize};

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
}
