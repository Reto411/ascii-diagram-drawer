//! Text emission (FR2): the resolved grid is the output.
//!
//! Written rule (concept C2, emit stage): rows are joined with `'\n'`, each row's
//! trailing whitespace is trimmed, there is no final newline, and the empty grid
//! exports as the empty string. The grid itself is never mutated — trimming is an
//! export concern; interior spaces are preserved exactly.

use render_pipeline::ResolvedGrid;

/// Exports a resolved grid as plain monospace text (UTF-8, LF line endings).
#[must_use]
pub fn to_text(grid: &ResolvedGrid) -> String {
    if grid.is_empty() {
        return String::new();
    }
    (0..grid.height())
        .map(|y| grid.row(y).expect("row in bounds").trim_end().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_grid_exports_as_empty_string() {
        assert_eq!(to_text(&ResolvedGrid::empty()), "");
    }

    #[test]
    fn rows_are_lf_joined_without_final_newline() {
        let grid = ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid");
        assert_eq!(to_text(&grid), "+---+\n| A |\n+---+");
    }

    #[test]
    fn trailing_whitespace_is_trimmed_per_line() {
        // Row 0 ends in spaces (the grid width is set by row 1's content) — trimmed;
        // interior spaces are preserved.
        let grid = ResolvedGrid::from_rows(&["hi      ", "abcdefgh"]).expect("valid grid");
        assert_eq!(to_text(&grid), "hi\nabcdefgh");
    }

    #[test]
    fn fully_empty_row_stays_empty_inside_the_grid() {
        let grid = ResolvedGrid::from_rows(&["abc", "   ", "def"]).expect("valid grid");
        assert_eq!(to_text(&grid), "abc\n\ndef");
    }

    #[test]
    fn svgbob_domain_input_is_preserved_exactly() {
        // Every printable-ASCII character the pipeline can emit survives byte-exact.
        let row = "+-.|:;/\\_`'()\"<^>vV abc019";
        let grid = ResolvedGrid::from_rows(&[row]).expect("valid grid");
        assert_eq!(to_text(&grid), row);
    }
}
