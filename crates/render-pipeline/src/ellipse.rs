//! Deterministic re-rasterization of the built-in ellipse at any instance extent (B-001).
//!
//! The natural 9×5 template is the reference rendering; every other size is derived from
//! it structurally, in the same glyph style:
//!
//! - a centered top cap of `_` and a bottom cap `` `-' `` whose length grows with the
//!   width (the natural cap is 3 of 9 columns),
//! - one diagonal row pair near the corners (`,'`/`` `. `` touching the left border,
//!   `'.`/`.'` inset one column from the right border — as in the template) when there
//!   is room for both a straight side row and non-overlapping diagonals,
//! - straight `(` / `)` side runs in between.
//!
//! [`rasterize_ellipse`] is a pure function of `(w, h)` (NFR3); `w = 9, h = 5` reproduces
//! the built-in template byte-for-byte, so natural-size rendering is unchanged.

use shape_registry::TemplateGrid;

/// Rasterizes an ellipse outline `w` × `h` cells wide/tall in the built-in glyph style.
///
/// # Panics
///
/// Never for `w, h ≥ 1` (the model rejects zero sizes); degenerate extents degrade to
/// their closest sensible form rather than failing.
pub(crate) fn rasterize_ellipse(w: u32, h: u32) -> TemplateGrid {
    debug_assert!(w >= 1 && h >= 1, "CellSize rejects zero extents");

    // Cap length ≈ width/3 (the natural cap is 3 of 9), ties up, clamped to [1, w].
    let cap = ((2 * w + 3) / 6).clamp(1, w);
    let x0 = (w - cap) / 2; // floor-centered: the natural 9-wide cap starts at column 3

    let mut rows: Vec<String> = Vec::with_capacity(h as usize);
    for y in 0..h {
        let mut row = vec![' '; w as usize];
        if h == 1 || y == 0 {
            // Top cap (also the whole shape when h == 1).
            for x in x0..x0 + cap {
                row[x as usize] = '_';
            }
        } else if y == h - 1 {
            // Bottom cap: backtick, dashes, apostrophe — the natural form is `` `-' ``.
            row[x0 as usize] = '`';
            for x in (x0 + 1)..(x0 + cap.saturating_sub(1)) {
                row[x as usize] = '-';
            }
            if cap >= 2 {
                row[(x0 + cap - 1) as usize] = '\'';
            }
        } else {
            // Interior rows: diagonal row pair near the corners when it fits (w ≥ 7 so
            // the two diagonals do not overlap, h ≥ 4 so at least one straight side row
            // exists), otherwise straight sides. The right pair is inset one column from
            // the border (the template's `'.`/`.'` end at column w-2).
            if w >= 7 && h >= 4 && y == 1 {
                row[1] = ',';
                row[2] = '\'';
                row[(w - 3) as usize] = '\'';
                row[(w - 2) as usize] = '.';
            } else if w >= 7 && h >= 4 && y == h - 2 {
                row[1] = '`';
                row[2] = '.';
                row[(w - 3) as usize] = '.';
                row[(w - 2) as usize] = '\'';
            } else if w >= 2 {
                row[0] = '(';
                row[(w - 1) as usize] = ')';
            }
        }
        rows.push(row.into_iter().collect());
    }

    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    TemplateGrid::from_rows(&refs).expect("rasterizer emits equal-length printable-ASCII rows")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The natural size is the reference rendering: byte-identical to the built-in art.
    #[test]
    fn natural_size_matches_the_builtin_template() {
        let reg = shape_registry::ShapeRegistry::builtin();
        let template = reg
            .lookup("ellipse")
            .expect("ellipse is built in")
            .glyph_pattern
            .clone();
        let raster = rasterize_ellipse(9, 5);
        assert_eq!(raster.width(), template.width());
        assert_eq!(raster.height(), template.height());
        for y in 0..template.height() {
            assert_eq!(
                raster.row(y),
                template.row(y),
                "row {y} diverges from the built-in template"
            );
        }
    }

    /// A pinned larger size: proportional cap, one diagonal row pair, straight sides.
    #[test]
    fn resized_13x8_is_a_wider_oval() {
        let raster = rasterize_ellipse(13, 8);
        let rows: Vec<String> = (0..raster.height())
            .map(|y| raster.row(y).unwrap())
            .collect();
        assert_eq!(
            rows,
            vec![
                "    ____     ",
                " ,'       '. ",
                "(           )",
                "(           )",
                "(           )",
                "(           )",
                " `.       .' ",
                "    `--'     ",
            ]
        );
    }

    /// The cap length is monotonic non-decreasing in the width and stays inside [1, w].
    #[test]
    fn cap_length_is_monotonic() {
        let cap = |w: u32| -> u32 {
            rasterize_ellipse(w, 5)
                .row(0)
                .unwrap()
                .chars()
                .filter(|c| *c == '_')
                .count() as u32
        };
        let mut prev = 0;
        for w in 1..=60u32 {
            let c = cap(w);
            assert!((1..=w).contains(&c), "cap {c} out of range at w={w}");
            assert!(c >= prev, "cap shrank from {prev} to {c} at w={w}");
            prev = c;
        }
    }

    /// Every extent in a wide box renders without panicking and stays inside its bounds:
    /// only the outline glyph vocabulary appears, on the expected rows/columns.
    #[test]
    fn exhaustive_extents_are_deterministic_and_in_bounds() {
        for w in 1..=40u32 {
            for h in 1..=25u32 {
                let raster = rasterize_ellipse(w, h);
                assert_eq!(raster.width(), w);
                assert_eq!(raster.height(), h);
                for y in 0..h {
                    let row = raster.row(y).unwrap();
                    for (x, c) in row.chars().enumerate() {
                        if c == ' ' {
                            continue;
                        }
                        let x = x as u32;
                        // Outline vocabulary only.
                        assert!(
                            matches!(c, '_' | '-' | '`' | '\'' | '(' | ')' | ',' | '.'),
                            "unexpected glyph {c:?} at ({x},{y}) in {w}x{h}"
                        );
                        // Art never on cells the rules do not claim: interior rows are
                        // only ever used at columns 0..=2 and (w-3)..(w-1); caps within [x0, x0+cap).
                        if y > 0 && y + 1 < h {
                            let edge_zone = x <= 2 || x >= w.saturating_sub(3);
                            assert!(edge_zone, "interior art at ({x},{y}) in {w}x{h}");
                        }
                    }
                }
            }
        }
    }

    /// Degenerate extents degrade gracefully (no panic, nothing out of bounds).
    #[test]
    fn degenerate_extents_do_not_panic() {
        for (w, h) in [(1u32, 1), (1, 7), (2, 1), (3, 4), (4, 5)] {
            let raster = rasterize_ellipse(w, h);
            assert_eq!(raster.width(), w);
            assert_eq!(raster.height(), h);
        }
    }
}
