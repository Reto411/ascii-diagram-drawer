//! Cell geometry — the integer cell grid value objects ([concept: cell grid](../../../doc/arc42/12-concepts/cell-grid.md)).
//!
//! All model coordinates are integer cell units; the origin `(0, 0)` is the top-left
//! corner of the grid and the canvas is unbounded in the positive x/y directions.

use crate::invariants::ModelError;

/// A position on the cell grid, in whole cells from the top-left origin (FR7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellPos {
    /// Column index (0-based, grows to the right).
    pub x: u32,
    /// Row index (0-based, grows downward).
    pub y: u32,
}

impl CellPos {
    /// Creates a position. Coordinates are unsigned by construction — negative
    /// positions are unrepresentable, which is how the grid invariant holds.
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    /// The grid origin `(0, 0)`.
    #[must_use]
    pub const fn origin() -> Self {
        Self::new(0, 0)
    }
}

/// A size in whole cells. Both dimensions are at least one — a zero-sized object
/// would render nothing and break the "every object occupies cells" invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellSize {
    /// Height in cells (≥ 1).
    pub height: u32,
    /// Width in cells (≥ 1).
    pub width: u32,
}

impl CellSize {
    /// Whether `pos` lies strictly inside a box of this size starting at the origin.
    /// Used for bounds checks (e.g. anchor cells inside their shape's bounds).
    #[must_use]
    pub const fn contains(self, pos: CellPos) -> bool {
        pos.x < self.width && pos.y < self.height
    }

    /// Creates a size, rejecting zero or negative dimensions.
    ///
    /// # Errors
    /// [`ModelError::ZeroSize`] if either dimension is zero.
    pub const fn new(width: u32, height: u32) -> Result<Self, ModelError> {
        if width == 0 || height == 0 {
            return Err(ModelError::ZeroSize { height, width });
        }
        Ok(Self { height, width })
    }

    /// The smallest possible size: one cell by one cell.
    #[must_use]
    pub const fn unit() -> Self {
        Self {
            height: 1,
            width: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "test code: panicking with the value on a failed construction is fine"
    )]

    use super::{CellPos, CellSize};
    use crate::invariants::ModelError;

    /// `new` builds coordinates; `origin` is (0, 0).
    #[test]
    fn cell_pos_new_and_origin() {
        assert_eq!(CellPos::new(3, 5), CellPos { x: 3, y: 5 });
        assert_eq!(CellPos::origin(), CellPos::new(0, 0));
    }

    /// `CellPos` is an immutable value type with equality by value.
    #[test]
    fn cell_pos_equality_is_by_value() {
        let first = CellPos::new(1, 2);
        let second = CellPos::new(1, 2);
        assert_eq!(first, second);
        assert_ne!(first, CellPos::new(2, 1));
    }

    /// Zero sizes are unrepresentable (unsigned) and rejected at construction.
    #[test]
    fn cell_size_new_rejects_zero_dimensions() {
        assert_eq!(
            CellSize::new(4, 2),
            Ok(CellSize {
                height: 2,
                width: 4
            })
        );
        assert_eq!(
            CellSize::new(0, 2),
            Err(ModelError::ZeroSize {
                height: 2,
                width: 0
            })
        );
        assert_eq!(
            CellSize::new(3, 0),
            Err(ModelError::ZeroSize {
                height: 0,
                width: 3
            })
        );
        assert_eq!(
            CellSize::new(0, 0),
            Err(ModelError::ZeroSize {
                height: 0,
                width: 0
            })
        );
    }

    /// `unit` is 1x1; `contains` is strict (the far edge is outside).
    #[test]
    fn cell_size_unit_and_contains() {
        let size = CellSize::unit();
        assert!(size.contains(CellPos::origin()));
        assert!(!size.contains(CellPos::new(1, 0)));
        assert!(!size.contains(CellPos::new(0, 1)));

        let big = CellSize::new(3, 2).expect("valid size");
        assert!(big.contains(CellPos::new(2, 1)));
        // The far edge is outside: contains is strict.
        assert!(!big.contains(CellPos::new(3, 0)));
        assert!(!big.contains(CellPos::new(0, 2)));
    }
}
