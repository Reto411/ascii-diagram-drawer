//! Pure grid↔screen mapping and cell hit-testing for the WYSIWYG canvas (S-012 AC2).
//!
//! No egui types here — plain `f32`/`u32` math, unit-testable without a window.

/// The canvas viewport: where cell `(0, 0)` starts on screen and how big one cell is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Pixel position of the top-left corner of cell `(0, 0)`, relative to the canvas area.
    pub origin_x: f32,
    /// Pixel position (vertical) of the top-left corner of cell `(0, 0)`.
    pub origin_y: f32,
    /// Pixels per grid cell (square).
    pub cell_px: f32,
}

impl Viewport {
    /// A viewport with a uniform margin around the grid and the given pixel size per cell.
    #[must_use]
    pub fn new(margin_px: f32, cell_px: f32) -> Self {
        Self {
            origin_x: margin_px,
            origin_y: margin_px,
            cell_px,
        }
    }

    /// The top-left pixel of the cell at `(col, row)`.
    #[must_use]
    pub fn cell_origin(&self, col: u32, row: u32) -> (f32, f32) {
        (
            self.origin_x + col as f32 * self.cell_px,
            self.origin_y + row as f32 * self.cell_px,
        )
    }

    /// The pixel rectangle `(x, y, w, h)` covering a `width`×`height` cell block starting at `(col, row)`.
    #[must_use]
    pub fn cell_block(&self, col: u32, row: u32, width: u32, height: u32) -> (f32, f32, f32, f32) {
        let (x, y) = self.cell_origin(col, row);
        (
            x,
            y,
            width as f32 * self.cell_px,
            height as f32 * self.cell_px,
        )
    }

    /// Hit-test: the model-space grid cell under a canvas-relative pixel position, or
    /// `None` if the position is before the origin (the margin region). Model space is
    /// unbounded — any non-negative cell is a valid placement target, so there is no
    /// extent argument.
    #[must_use]
    pub fn screen_to_model_cell(&self, x: f32, y: f32) -> Option<(u32, u32)> {
        let col = ((x - self.origin_x) / self.cell_px).floor();
        let row = ((y - self.origin_y) / self.cell_px).floor();
        if col < 0.0 || row < 0.0 {
            return None;
        }
        Some((col as u32, row as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp() -> Viewport {
        // 8 px margin, 16 px cells.
        Viewport::new(8.0, 16.0)
    }

    #[test]
    fn cell_origin_lands_on_the_grid() {
        let vp = vp();
        assert_eq!(vp.cell_origin(0, 0), (8.0, 8.0));
        assert_eq!(vp.cell_origin(3, 2), (8.0 + 48.0, 8.0 + 32.0));
    }

    #[test]
    fn cell_block_covers_exactly_the_requested_cells() {
        let vp = vp();
        assert_eq!(vp.cell_block(1, 2, 9, 5), (24.0, 40.0, 144.0, 80.0));
    }

    #[test]
    fn hit_test_round_trips_every_cell() {
        let vp = vp();
        for col in 0..12u32 {
            for row in 0..7u32 {
                // Probe the middle of each cell.
                let (x, y) = vp.cell_origin(col, row);
                assert_eq!(
                    vp.screen_to_model_cell(x + 8.0, y + 8.0),
                    Some((col, row)),
                    "cell ({col}, {row})"
                );
            }
        }
    }

    #[test]
    fn hit_test_is_inclusive_of_the_top_left_edge_and_exclusive_of_the_right_bottom() {
        let vp = vp();
        // Top-left corner of cell (2, 1) is inside it…
        assert_eq!(vp.screen_to_model_cell(40.0, 24.0), Some((2, 1)));
        // …but the right/bottom edge belongs to the next cell.
        assert_eq!(vp.screen_to_model_cell(72.0, 24.0), Some((4, 1)));
        assert_eq!(vp.screen_to_model_cell(40.0, 40.0), Some((2, 2)));
    }

    #[test]
    fn hit_test_is_unbounded_but_rejects_the_margin_region() {
        let vp = vp();
        // Far beyond any content — still a valid model cell.
        assert_eq!(
            vp.screen_to_model_cell(10_000.0, 10_000.0),
            Some((624, 624))
        );
        // Left/above the grid origin (margin region) is not.
        assert_eq!(vp.screen_to_model_cell(7.0, 7.0), None);
        assert_eq!(vp.screen_to_model_cell(-1.0, 40.0), None);
    }
}
