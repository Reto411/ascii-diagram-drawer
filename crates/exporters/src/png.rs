//! PNG emission (FR5): rasterization of the SVG produced by [`crate::to_svg`] —
//! ADR 005 delegates the grid→vector step to svgbob, [ADR 016](../../../../doc/adr/016-png-rasterizer.md)
//! delegates the vector→raster step to `resvg` (usvg parse → tiny-skia pixmap → PNG encode).
//!
//! The rasterizer is a build-time concern: behind the default `raster` feature. Builds
//! without it keep every other export working and get [`crate::ExportError::Unsupported`]
//! from [`to_png`] — degradation, not failure (AC3).

use render_pipeline::ResolvedGrid;

use crate::{ExportError, to_svg};

/// Exports a resolved grid as a PNG image (RGBA, 8 bits per channel).
///
/// The grid is first exported as SVG ([`to_svg`]); the rasterizer renders that exact
/// document at its declared size, so the PNG always matches the SVG and text outputs.
pub fn to_png(grid: &ResolvedGrid) -> Result<Vec<u8>, ExportError> {
    let svg = to_svg(grid)?;
    #[cfg(feature = "raster")]
    {
        png_from_svg(&svg)
    }
    #[cfg(not(feature = "raster"))]
    {
        let _ = svg;
        Err(ExportError::Unsupported)
    }
}

/// The rasterizer seam (ADR 016): usvg parse → resvg render into a tiny-skia pixmap at
/// the document's declared size → PNG encode. Every fallible step maps to a typed
/// [`ExportError`] — nothing in this path panics.
#[cfg(feature = "raster")]
fn png_from_svg(svg: &str) -> Result<Vec<u8>, ExportError> {
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).map_err(|err| {
        ExportError::SvgParse {
            message: err.to_string(),
        }
    })?;
    // The document's declared pixel size (f32 in usvg) — the PNG renders at exactly
    // this size, so decoded dimensions equal the SVG's width/height attributes.
    let size = tree.size();
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).ok_or_else(|| {
            ExportError::PngEncode {
                message: "zero-size SVG document".into(),
            }
        })?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|err| ExportError::PngEncode {
        message: err.to_string(),
    })
}

#[cfg(all(test, feature = "raster"))]
mod tests {
    use super::*;
    use crate::ExportError;
    use render_pipeline::ResolvedGrid;

    fn small_grid() -> ResolvedGrid {
        ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid")
    }

    /// AC2 — the bytes are a real PNG: signature verified, and a decoder round-trips
    /// dimensions that match the SVG's declared size (faithful rasterization).
    #[test]
    fn png_signature_and_decoded_dimensions_match_the_svg() {
        let grid = small_grid();
        let svg = to_svg(&grid).expect("renders");
        let bytes = to_png(&grid).expect("rasterizes");

        assert_eq!(
            &bytes[..8],
            b"\x89PNG\r\n\x1a\n",
            "must start with the PNG signature"
        );

        // Decode with the `png` crate (dev-dependency) — a real decoder round-trip.
        let decoder = png::Decoder::new(&bytes[..]);
        let reader = decoder.read_info().expect("decodable PNG");
        let info = reader.info();
        assert_eq!(info.bit_depth, png::BitDepth::Eight);

        // The SVG declares its own pixel size; the pixmap is created at exactly that
        // size, so decoded dimensions must equal it.
        let declared = |attr: &str| -> u32 {
            let start = svg
                .find(format!("{attr}=\"").as_str())
                .expect("attribute present");
            let rest = &svg[start + attr.len() + 2..];
            rest.split('"')
                .next()
                .expect("value")
                .parse()
                .expect("numeric")
        };
        assert_eq!(info.width, declared("width"));
        assert_eq!(info.height, declared("height"));
    }

    /// AC4 — deterministic bytes: same grid → byte-identical PNG across calls.
    #[test]
    fn reexporting_the_same_grid_is_byte_identical() {
        let grid = small_grid();
        assert_eq!(
            to_png(&grid).expect("rasterizes"),
            to_png(&grid).expect("rasterizes")
        );
    }

    /// AC2/AC3 — the error seam: a document usvg cannot parse surfaces as a typed
    /// `SvgParse` error (never a panic), and `Unsupported` is constructible with a
    /// clear message.
    #[test]
    fn raster_error_paths_are_typed() {
        let err = png_from_svg("definitely not an svg <").expect_err("must fail");
        assert!(matches!(err, ExportError::SvgParse { .. }), "got {err:?}");

        let unsupported = ExportError::Unsupported;
        assert!(unsupported.to_string().contains("no rasterizer backend"));
    }

    /// A grid with connector arrowheads rasterizes too (the S-008 golden scenario).
    #[test]
    fn connector_grid_rasterizes() {
        let grid = ResolvedGrid::from_rows(&[
            "+-------+     +-------+",
            "|       |     |       |",
            "|   A   +---->+   B   |",
            "|       |     |       |",
            "+-------+     +-------+",
        ])
        .expect("valid grid");
        let bytes = to_png(&grid).expect("rasterizes");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
}

#[cfg(test)]
mod unsupported_tests {
    /// AC3 — `ExportError::Unsupported` is constructible and covered regardless of
    /// build configuration (the no-`raster` path itself is exercised by running the
    /// test suite with `--no-default-features`).
    /// AC3 — without the `raster` feature, `to_png` degrades to `Unsupported`
    /// instead of breaking the build (text/SVG exports stay available).
    #[cfg(not(feature = "raster"))]
    #[test]
    fn to_png_degrades_to_unsupported_without_the_raster_feature() {
        let grid = render_pipeline::ResolvedGrid::from_rows(&["+---+"]).expect("valid grid");
        let err = crate::to_png(&grid).expect_err("must degrade, not build-fail");
        assert!(matches!(err, crate::ExportError::Unsupported));
    }

    #[test]
    fn unsupported_variant_is_constructible() {
        let err = crate::ExportError::Unsupported;
        assert!(err.to_string().contains("no rasterizer backend"));
    }
}
