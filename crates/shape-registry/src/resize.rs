//! Re-rasterization of rule-bearing templates at instance sizes (C10, ADR 007 v6).
//!
//! [`rasterize`] is a **pure function of (art, rules, target size)** — no IO, fully
//! deterministic (NFR3). The mechanism: the template's four corner cells are fixed;
//! only the runs between them stretch. Width changes act on every row's run just
//! before its last cell ("before the right corner"); height changes add or remove
//! whole rows just above the bottom row. See [resize-rules](../../../doc/arc42/12-concepts/resize-rules.md)
//! for the normative behavior and the finalized statement grammar (S-024).

use crate::error::ResizeError;
use crate::geometry::CellSize;
use crate::types::{ResizeRules, TemplateGrid};

/// Re-rasterizes `grid` under `rules` at the target `width × height`.
///
/// Identity: the target equal to the natural size returns the template itself.
/// Growth never fails; shrink stops with [`ResizeError`] when it would cross the
/// minimum extent or remove a cell that is not the declared/auto-derived remove
/// character (art preservation — corners and non-uniform rows are never destroyed).
///
/// # Errors
/// [`ResizeError::BelowMinimum`] when the target (or a shrink step) goes below the
/// minimum reachable extent, [`ResizeError::RuleMismatch`] / [`ResizeError::RowContainsArt`]
/// when a shrink step would remove art that the rules do not cover.
pub fn rasterize(
    grid: &TemplateGrid,
    rules: &ResizeRules,
    width: u32,
    height: u32,
) -> Result<TemplateGrid, ResizeError> {
    let (w, h) = (grid.width(), grid.height());
    if width == w && height == h {
        return Ok(grid.clone());
    }

    // ── Step 1: width w → W (each row keeps its first and last cells) ──────────
    let mut rows: Vec<Vec<char>> = (0..h)
        .map(|y| grid.row(y).expect("in-range row").chars().collect())
        .collect();

    if width != w {
        let delta = width.abs_diff(w);
        let growing = width > w;
        for y in 0..h {
            let row = &mut rows[y as usize];
            // Row characters: top/bottom edges from the rules, interior rows from
            // the art itself — the cell just inside the right border is the fill
            // that run already uses (space for bordered shapes, the fill glyph for
            // solid ones). A 1-row template is its own top edge.
            let (ins, rem) = if h == 1 || y == 0 {
                rules.top.chars_for(width)
            } else if y == h - 1 {
                rules.bottom.chars_for(width)
            } else {
                let fill = row[w.saturating_sub(2) as usize];
                (fill, fill)
            };
            if growing {
                if w == 1 {
                    // Degenerate: no run exists — extend to the right of the single cell.
                    for _ in 0..delta {
                        row.push(ins);
                    }
                } else {
                    let insert_at = (w - 1) as usize; // just before the last cell
                    for _ in 0..delta {
                        row.insert(insert_at, ins);
                    }
                }
            } else {
                if delta > w.saturating_sub(2) {
                    return Err(ResizeError::BelowMinimum { width, height });
                }
                let last = *row.last().expect("non-empty row");
                for k in 1..=delta {
                    let idx = (w - 1 - k) as usize;
                    if row[idx] != rem {
                        return Err(ResizeError::RuleMismatch {
                            row: y,
                            column: idx as u32,
                            expected: rem,
                            found: row[idx],
                        });
                    }
                }
                row.truncate((w - 1 - delta) as usize);
                row.push(last);
            }
        }
    }

    let w_adj = width; // rows are now h × W

    // ── Step 2: height h → H (rows added/removed just above the bottom row) ────
    if height != h {
        if height > h {
            let delta = height - h;
            let (left_ch, _) = rules.left.chars_for(height);
            let (_, right_ch) = rules.right.chars_for(height);
            // New rows take their fill from the bottommost original row that is not
            // the bottom corner row (its inside-right cell) — the top row's when the
            // art is exactly two rows, plain space for 1-row/1-wide degenerate art.
            let fill = if h >= 3 && w_adj >= 2 {
                rows[(h - 2) as usize][(w_adj - 2) as usize]
            } else if h == 2 && w_adj >= 2 {
                rows[0][(w_adj - 2) as usize]
            } else {
                ' '
            };
            let bottom = rows.pop().expect("non-empty grid");
            for _ in 0..delta {
                let mut new_row = Vec::with_capacity(w_adj as usize);
                new_row.push(left_ch);
                if w_adj >= 2 {
                    for _ in 0..w_adj - 2 {
                        new_row.push(fill);
                    }
                    new_row.push(right_ch);
                }
                rows.push(new_row);
            }
            rows.push(bottom);
        } else {
            let delta = h - height;
            if delta > h.saturating_sub(2) {
                return Err(ResizeError::BelowMinimum { width, height });
            }
            let (left_rem, _) = rules.left.chars_for(height);
            let (_, right_rem) = rules.right.chars_for(height);
            for r in (0..delta).rev() {
                // Remove rows h-2, h-3, … h-1-delta — each must be pure edge+fill.
                let row = &rows[(h - 2 - r) as usize];
                if row.first().copied() != Some(left_rem) {
                    return Err(ResizeError::RuleMismatch {
                        row: h - 2 - r,
                        column: 0,
                        expected: left_rem,
                        found: row[0],
                    });
                }
                if row.last().copied() != Some(right_rem) {
                    return Err(ResizeError::RuleMismatch {
                        row: h - 2 - r,
                        column: w_adj - 1,
                        expected: right_rem,
                        found: row[(w_adj - 1) as usize],
                    });
                }
                let interior = if w_adj >= 2 {
                    &row[1..(w_adj - 1) as usize]
                } else {
                    &[]
                };
                if interior.iter().any(|&c| c != interior[0]) {
                    return Err(ResizeError::RowContainsArt { row: h - 2 - r });
                }
            }
            for _ in 0..delta {
                // Always remove the row just above the (fixed) bottom row; after
                // each removal the list shrinks, so recompute the index from the
                // current length.
                let len = rows.len();
                rows.remove(len - 2);
            }
        }
    }

    let out: Vec<String> = rows.iter().map(|r| r.iter().collect()).collect();
    Ok(
        TemplateGrid::from_rows(&out.iter().map(String::as_str).collect::<Vec<_>>())
            .expect("rasterized cells are printable ASCII by construction"),
    )
}

/// The smallest size reachable from the template's natural size by repeated valid
/// single-step shrinks (per axis, independently) — the lower bound for resize
/// gestures and render-time clamping. Always ≥ 1×1; rule-bearing templates shrink
/// no further than their corners and declared remove characters allow.
#[must_use]
pub fn min_size(grid: &TemplateGrid, rules: &ResizeRules) -> CellSize {
    let (w, h) = (grid.width(), grid.height());
    let mut min_w = w;
    while min_w > 1 && rasterize(grid, rules, min_w - 1, h).is_ok() {
        min_w -= 1;
    }
    let mut min_h = h;
    while min_h > 1 && rasterize(grid, rules, w, min_h - 1).is_ok() {
        min_h -= 1;
    }
    CellSize::new(min_w, min_h)
}

#[cfg(test)]
mod tests {
    use super::{min_size, rasterize};
    use crate::error::ResizeError;
    use crate::geometry::CellSize;
    use crate::types::{EdgeRule, ParityChars, ResizeRules, TemplateGrid};

    /// The canonical box: `+--+` / `|  |` / `+--+` with the matching rule set.
    fn box_grid() -> TemplateGrid {
        TemplateGrid::from_rows(&["+--+", "|  |", "+--+"]).expect("valid")
    }

    fn box_rules() -> ResizeRules {
        ResizeRules {
            top: EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            bottom: EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            left: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
            right: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
        }
    }

    fn rows(grid: &TemplateGrid) -> Vec<String> {
        (0..grid.height()).map(|y| grid.row(y).unwrap()).collect()
    }

    /// Identity: the natural size returns the template byte-for-byte.
    #[test]
    fn rasterize_is_identity_at_natural_size() {
        let g = box_grid();
        let out = rasterize(&g, &box_rules(), 4, 3).expect("identity");
        assert_eq!(out, g);
    }

    /// Box width growth: top/bottom widen with `-` before the right corner, interior
    /// rows gain space fill.
    #[test]
    fn box_width_growth_stretches_runs() {
        let out = rasterize(&box_grid(), &box_rules(), 6, 3).expect("grow");
        assert_eq!(rows(&out), vec!["+----+", "|    |", "+----+"]);
    }

    /// Box height growth: new rows are `|` + space fill + `|`, inserted above the
    /// bottom row.
    #[test]
    fn box_height_growth_adds_interior_rows() {
        let out = rasterize(&box_grid(), &box_rules(), 4, 5).expect("grow");
        assert_eq!(rows(&out), vec!["+--+", "|  |", "|  |", "|  |", "+--+"]);
    }

    /// Combined growth on both axes composes (width first, then height).
    #[test]
    fn box_combined_growth() {
        let out = rasterize(&box_grid(), &box_rules(), 7, 6).expect("grow");
        assert_eq!(
            rows(&out),
            vec![
                "+-----+".to_owned(),
                "|     |".to_owned(),
                "|     |".to_owned(),
                "|     |".to_owned(),
                "|     |".to_owned(),
                "+-----+".to_owned()
            ]
        );
    }

    /// Box shrink: one step per axis removes exactly the declared characters.
    #[test]
    fn box_shrink_one_step_per_axis() {
        let g = box_grid();
        let narrow = rasterize(&g, &box_rules(), 3, 3).expect("shrink w");
        assert_eq!(rows(&narrow), vec!["+-+", "| |", "+-+"]);
        let short = rasterize(&g, &box_rules(), 4, 2).expect("shrink h");
        // Only the bottom interior row (row 1) is removable — it is pure edge+fill.
        assert_eq!(rows(&short), vec!["+--+", "+--+"]);
    }

    /// Width 2 (empty run between corners) is reachable; width/height 1 would
    /// collapse the corner cells and is rejected, not clipped.
    #[test]
    fn shrink_below_minimum_is_rejected() {
        let g = box_grid();
        let at_two = rasterize(&g, &box_rules(), 2, 3).expect("width 2 ok");
        assert_eq!(rows(&at_two), vec!["++", "||", "++"]);
        assert_eq!(
            rasterize(&g, &box_rules(), 1, 3),
            Err(ResizeError::BelowMinimum {
                width: 1,
                height: 3
            })
        );
        assert_eq!(
            rasterize(&g, &box_rules(), 4, 1),
            Err(ResizeError::BelowMinimum {
                width: 4,
                height: 1
            })
        );
    }

    /// A shrink step whose removed cell differs from the declared remove character is
    /// a typed mismatch — art inside the bottom edge run cannot be eaten.
    #[test]
    fn shrink_removing_undeclared_char_is_rejected() {
        let g = TemplateGrid::from_rows(&["+--+", "|  |", "+-x+"]).expect("valid");
        assert_eq!(
            rasterize(&g, &box_rules(), 3, 3),
            Err(ResizeError::RuleMismatch {
                row: 2,
                column: 2,
                expected: '-',
                found: 'x',
            })
        );
    }

    /// Height shrink removes rows just above the bottom; a row containing art is
    /// protected.
    #[test]
    fn height_shrink_protects_art_rows() {
        let g = TemplateGrid::from_rows(&["+--+", "|  |", "|a |", "+--+"]).expect("valid");
        // Removing row 2 (`|a |` — non-uniform interior) is rejected…
        assert_eq!(
            rasterize(&g, &box_rules(), 4, 3),
            Err(ResizeError::RowContainsArt { row: 2 })
        );
        // …but a pure edge+fill row is removed.
        let uniform = TemplateGrid::from_rows(&["+--+", "|  |", "|  |", "+--+"]).expect("valid");
        let out = rasterize(&uniform, &box_rules(), 4, 3).expect("shrink h");
        assert_eq!(rows(&out), vec!["+--+", "|  |", "+--+"]);
    }

    /// Parity: the even-width override is selected by the TARGET extent's parity.
    #[test]
    fn parity_variants_follow_target_extent() {
        let g = box_grid();
        let mut rules = box_rules();
        rules.top.even = Some(ParityChars {
            insert: '=',
            remove: '=',
        });
        // The parity override selects the character for *inserted* cells only —
        // existing art is never rewritten (C10): even target width 6 inserts `==`
        // just before the corner, odd target width 5 inserts base `-`.
        let even = rasterize(&g, &rules, 6, 3).expect("even");
        assert_eq!(even.row(0), Some("+--==+".to_owned()));
        let odd = rasterize(&g, &rules, 5, 3).expect("odd");
        assert_eq!(odd.row(0), Some("+---+".to_owned()));
    }

    /// min_size: the box shrinks to 2×2 (empty runs) and no further.
    #[test]
    fn min_size_of_box_is_two_by_two() {
        assert_eq!(min_size(&box_grid(), &box_rules()), CellSize::new(2, 2));
    }

    /// min_size stops where the remove characters stop matching — the slanted row's
    /// ends are not the declared left/right remove characters, so height shrink fails
    /// after one step.
    #[test]
    fn min_size_stops_at_art_boundaries() {
        let g = TemplateGrid::from_rows(&["+---+", "/   /", "(   )", "+---+"]).expect("valid");
        let rules = ResizeRules {
            top: EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            bottom: EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            left: EdgeRule {
                insert: '(',
                remove: '(',
                even: None,
                odd: None,
            },
            right: EdgeRule {
                insert: ')',
                remove: ')',
                even: None,
                odd: None,
            },
        };
        // Width shrinks to 2 (auto-derived space fill); height only one step — the
        // `(   )` row is removable, then `/   /`'s ends (`/`) are not `(`/`)`.
        let min = min_size(&g, &rules);
        assert_eq!(min.width, 2);
        assert_eq!(min.height, 3);
    }

    /// A solid template (no space fill) grows with its own fill character.
    #[test]
    fn solid_template_uses_own_fill() {
        let g = TemplateGrid::from_rows(&["####", "####"]).expect("valid");
        let rules = ResizeRules {
            top: EdgeRule {
                insert: '#',
                remove: '#',
                even: None,
                odd: None,
            },
            bottom: EdgeRule {
                insert: '#',
                remove: '#',
                even: None,
                odd: None,
            },
            left: EdgeRule {
                insert: '#',
                remove: '#',
                even: None,
                odd: None,
            },
            right: EdgeRule {
                insert: '#',
                remove: '#',
                even: None,
                odd: None,
            },
        };
        let out = rasterize(&g, &rules, 6, 3).expect("grow");
        assert_eq!(rows(&out), vec!["######", "######", "######"]);
    }

    /// Degenerate 1-row template: width growth appends to the right; height growth
    /// builds rows from the edge characters.
    #[test]
    fn degenerate_single_row_template() {
        let g = TemplateGrid::from_rows(&["---"]).expect("valid");
        let rules = ResizeRules {
            top: EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            bottom: EdgeRule {
                insert: '-',
                remove: '-',
                even: None,
                odd: None,
            },
            left: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
            right: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
        };
        // The single row is the bottom row; new rows go just above it.
        let out = rasterize(&g, &rules, 5, 2).expect("grow");
        assert_eq!(rows(&out), vec!["|   |", "-----"]);
    }

    /// Degenerate 1-wide template: height growth keeps rows 1 cell wide (the left
    /// edge character wins — there is no room for a right end).
    #[test]
    fn degenerate_single_column_template() {
        let g = TemplateGrid::from_rows(&["|", "|", "|"]).expect("valid");
        let rules = ResizeRules {
            top: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
            bottom: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
            left: EdgeRule {
                insert: '|',
                remove: '|',
                even: None,
                odd: None,
            },
            right: EdgeRule {
                insert: 'x',
                remove: 'x',
                even: None,
                odd: None,
            },
        };
        let out = rasterize(&g, &rules, 1, 4).expect("grow h");
        assert_eq!(rows(&out), vec!["|", "|", "|", "|"]);
    }
}
