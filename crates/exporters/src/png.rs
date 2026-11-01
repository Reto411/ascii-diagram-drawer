//! PNG emission (FR5): rasterization of the SVG produced by [`crate::to_svg`] —
//! ADR 005 delegates the grid→vector step to svgbob, [ADR 016](../../../../doc/adr/016-png-rasterizer.md)
//! delegates the vector→raster step to `resvg` (usvg parse → tiny-skia pixmap → PNG encode).
//!
//! The rasterizer is a build-time concern: behind the default `raster` feature. Builds
//! without it keep every other export working and get [`crate::ExportError::Unsupported`]
//! from [`to_png`] — degradation, not failure (AC3).

use render_pipeline::ResolvedGrid;

use crate::{ExportError, SvgOptions, to_svg_with_options};

/// Exports a resolved grid as a PNG image (RGBA, 8 bits per channel).
///
/// The grid is first exported as SVG ([`crate::to_svg`]); the rasterizer renders that exact
/// document at its declared size, so the PNG always matches the SVG and text outputs.
pub fn to_png(grid: &ResolvedGrid) -> Result<Vec<u8>, ExportError> {
    to_png_with_options(grid, &SvgOptions::default())
}

/// Exports a resolved grid as a PNG image with per-media color overrides (FR15):
/// the SVG is produced by [`to_svg_with_options`] and rasterized exactly like
/// [`to_png`], so a colored PNG always matches its colored SVG.
pub fn to_png_with_options(
    grid: &ResolvedGrid,
    options: &SvgOptions,
) -> Result<Vec<u8>, ExportError> {
    let svg = to_svg_with_options(grid, options)?;
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

/// A raw RGBA raster (row-major, top to bottom) — the in-memory result of
/// rasterization before encoding. [`to_png`] encodes one to a file; the GUI's live
/// preview pane (CR-004 / S-026) displays one directly as a texture via
/// [`text_to_rgba`], so what the user sees is exactly what the export produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in pixels (the SVG document's declared width).
    pub width: u32,
    /// Height in pixels (the SVG document's declared height).
    pub height: u32,
    /// `width * height` RGBA bytes, row-major, top to bottom.
    pub rgba: Vec<u8>,
}

/// Live-preview rasterization (CR-004 / S-026): render an exact svgbob input string
/// to raw RGBA — the same [`crate::svg::svg_from_text_with_options`] + resvg pipeline as
/// [`to_svg`] + [`to_png`], but taking the text bytes directly so the GUI can preview
/// from the canvas frame it already holds. The pixels are exactly what exporting
/// those bytes would produce: [`to_svg`] is defined as the default-options render of `&to_text(grid)`,
/// and this function funnels through the same seam.
///
/// Degrades to [`ExportError::Unsupported`] without the `raster` feature, like
/// [`to_png`] (AC4 — degradation, not failure).
pub fn text_to_rgba(text: &str) -> Result<RgbaImage, ExportError> {
    text_to_rgba_with_options(text, &SvgOptions::default())
}

/// Live-preview rasterization with per-media color overrides (FR15): the same pipeline
/// as [`text_to_rgba`], but the SVG is produced by
/// [`crate::svg::svg_from_text_with_options`], so a colored preview is exactly what a
/// colored export of the same bytes would produce.
pub fn text_to_rgba_with_options(
    text: &str,
    options: &SvgOptions,
) -> Result<RgbaImage, ExportError> {
    let svg = crate::svg::svg_from_text_with_options(text, options)?;
    #[cfg(feature = "raster")]
    {
        rgba_from_svg(&svg)
    }
    #[cfg(not(feature = "raster"))]
    {
        let _ = svg;
        Err(ExportError::Unsupported)
    }
}

/// The rasterizer seam (ADR 016): usvg parse → resvg render into a tiny-skia pixmap at
/// the document's declared size. Every fallible step maps to a typed [`ExportError`] —
/// nothing in this path panics.
#[cfg(feature = "raster")]
fn render_pixmap(svg: &str) -> Result<tiny_skia::Pixmap, ExportError> {
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).map_err(|err| {
        ExportError::SvgParse {
            message: err.to_string(),
        }
    })?;
    // The document's declared pixel size (f32 in usvg) — the raster renders at exactly
    // this size, so output dimensions equal the SVG's width/height attributes.
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
    Ok(pixmap)
}

#[cfg(feature = "raster")]
fn rgba_from_svg(svg: &str) -> Result<RgbaImage, ExportError> {
    let pixmap = render_pixmap(svg)?;
    // `take()` un-premultiplies and hands over the raw RGBA bytes.
    Ok(RgbaImage {
        width: pixmap.width(),
        height: pixmap.height(),
        rgba: pixmap.take(),
    })
}

/// The PNG encode step of the rasterizer seam (ADR 016): render, then encode with
/// tiny-skia's PNG encoder — the same direct pixmap→encode path as before S-026,
/// so export bytes are unchanged.
#[cfg(feature = "raster")]
fn png_from_svg(svg: &str) -> Result<Vec<u8>, ExportError> {
    let pixmap = render_pixmap(svg)?;
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
        let svg = crate::to_svg(&grid).expect("renders");
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

    /// AC1 (S-026) — the preview seam: rasterizing the exact canvas text yields
    /// byte-identical pixels to decoding the PNG export of the same grid — same SVG
    /// bytes in, same resvg pipeline, so the GUI's preview is exactly what FR5 exports.
    #[test]
    fn text_to_rgba_matches_the_png_export_pixels() {
        let grid = small_grid();
        let text = crate::to_text(&grid);
        let rgba = text_to_rgba(&text).expect("rasterizes");

        // Dimensions equal the SVG's declared size (same invariant as the PNG test).
        let svg = crate::to_svg(&grid).expect("renders");
        let declared = |attr: &str| -> u32 {
            let start = svg
                .find(format!("{attr}=").as_str())
                .expect("attribute present");
            let rest = &svg[start + attr.len() + 2..];
            rest.split('"')
                .next()
                .expect("value")
                .parse()
                .expect("numeric")
        };
        assert_eq!(rgba.width, declared("width"));
        assert_eq!(rgba.height, declared("height"));
        assert_eq!(rgba.rgba.len(), (rgba.width * rgba.height * 4) as usize);

        // Same pixels as the decoded PNG export of the same grid.
        let png_bytes = to_png(&grid).expect("rasterizes");
        let decoder = png::Decoder::new(&png_bytes[..]);
        let mut reader = decoder.read_info().expect("decodable PNG");
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
        let mut decoded = vec![0u8; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut decoded).expect("decodes");
        assert_eq!(frame.buffer_size(), rgba.rgba.len());
        assert_eq!(
            &decoded[..frame.buffer_size()],
            &rgba.rgba,
            "preview ≠ export pixels"
        );
    }

    /// AC1 (S-026) — deterministic: the same text rasterizes to identical bytes.
    #[test]
    fn text_to_rgba_is_deterministic() {
        let grid = small_grid();
        let text = crate::to_text(&grid);
        assert_eq!(
            text_to_rgba(&text).expect("rasterizes"),
            text_to_rgba(&text).expect("rasterizes")
        );
    }

    /// AC4 (S-026) — empty input never panics: svgbob accepts the empty document and
    /// rasterizes it to its default-size blank canvas. The *GUI* gates on empty canvas
    /// text and shows its placeholder instead of displaying this blank raster.
    #[test]
    fn text_to_rgba_handles_the_empty_document() {
        let img = text_to_rgba("").expect("blank document, not an error");
        assert!(img.width > 0 && img.height > 0);
        assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
    }

    /// FR15 (CR-009) — the colored PNG path: the preview seam over the same text and
    /// options yields exactly the pixels of the decoded colored PNG export (the S-026
    /// WYSIWYG invariant, now with colors), and the backdrop rasterizes to the
    /// background role's color.
    #[test]
    fn png_with_options_matches_the_colored_preview_pixels() {
        let grid = ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid");
        let options = crate::SvgOptions {
            background: Some("#202124".into()),
            fill: Some("#8ab4f8".into()),
            stroke: Some("#e8eaed".into()),
        };

        let png_bytes = to_png_with_options(&grid, &options).expect("rasterizes");
        assert_eq!(&png_bytes[..8], b"\x89PNG\r\n\x1a\n");

        let rgba = text_to_rgba_with_options(&crate::to_text(&grid), &options).expect("rasterizes");
        let decoder = png::Decoder::new(&png_bytes[..]);
        let mut reader = decoder.read_info().expect("decodable PNG");
        let mut decoded = vec![0u8; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut decoded).expect("decodes");
        assert_eq!(
            &decoded[..frame.buffer_size()],
            &rgba.rgba,
            "preview ≠ export pixels"
        );

        // Top-left corner sits on the backdrop rect → the background role's color.
        assert_eq!(
            [rgba.rgba[0], rgba.rgba[1], rgba.rgba[2]],
            [0x20, 0x21, 0x24],
            "backdrop rasterizes to the background role"
        );
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

    /// AC4 (S-026) — without the `raster` feature the preview seam degrades to
    /// `Unsupported` too, so the GUI shows its "unavailable" message instead.
    #[cfg(not(feature = "raster"))]
    #[test]
    fn text_to_rgba_degrades_to_unsupported_without_the_raster_feature() {
        let err = crate::text_to_rgba("+---+").expect_err("must degrade, not build-fail");
        assert!(matches!(err, crate::ExportError::Unsupported));
    }

    #[test]
    fn unsupported_variant_is_constructible() {
        let err = crate::ExportError::Unsupported;
        assert!(err.to_string().contains("no rasterizer backend"));
    }
}
