//! Headless software screenshots of egui frames — tier 2 without a GPU.
//!
//! The devcontainer cannot create a wgpu surface (no EGL; apt is blocked by
//! the ADR 014 egress allow-list), so live `xvfb` captures are impossible here.
//! This module closes that gap: it takes a pumped [`egui::FullOutput`] plus the
//! context's font atlas, tessellates with epaint's own tessellator (exactly
//! what the GPU backend would receive) and rasterizes the triangles into an
//! RGBA buffer in pure Rust. No window, no GL, no X11.
//!
//! Used by `gui_tests::visual_dump` (opt-in via `GUI_DUMP=1`) to produce PNGs
//! for visual inspection and future golden-screenshot tests.
//!
//! Conventions: epaint vertex colors are stored **straight** sRGBA (the
//! tessellator never premultiplies), and the font atlas holds white glyphs
//! whose alpha channel is the coverage. This rasterizer therefore uses plain
//! straight-alpha "over" compositing — `src.rgb*src_a + dst.rgb*(1-src_a)` —
//! which is exact for opaque shapes (the overwhelming majority of egui UI)
//! and more accurate than the GPU backend's approximation for translucent
//! ones. Feathering is disabled (fewer triangles; text anti-aliasing comes
//! from the glyph atlas anyway).

use eframe::egui;
use egui::epaint::tessellator::{TessellationOptions, Tessellator};
use egui::epaint::{Mesh, Primitive, TextureId, Vertex};
use egui::{Color32, ColorImage, FullOutput};
use std::path::Path;

/// Rasterize `out` (the last pumped frame of `ctx`) to a PNG at `path`.
///
/// Returns the image size in pixels.
pub fn capture_to_png(
    ctx: &egui::Context,
    out: &FullOutput,
    width: u32,
    height: u32,
    path: &Path,
) -> Result<(u32, u32), String> {
    let font_image = ctx.fonts(|f| f.image());
    let font_size = ctx.fonts(|f| f.font_image_size());

    let options = TessellationOptions {
        feathering: false,
        ..Default::default()
    };
    let mut tessellator = Tessellator::new(out.pixels_per_point, options, font_size, Vec::new());
    let meshes = tessellator.tessellate_shapes(out.shapes.clone());

    // Straight-alpha RGBA8 framebuffer.
    let mut buf: Vec<[u8; 4]> = vec![[0, 0, 0, 0]; (width * height) as usize];
    for prim in &meshes {
        if let Primitive::Mesh(mesh) = &prim.primitive {
            rasterize_mesh(&mut buf, width, height, prim.clip_rect, mesh, &font_image);
        }
    }

    // The framebuffer is already straight-alpha — write it out directly.
    let mut bytes = Vec::with_capacity(buf.len() * 4);
    for [r, g, b, a] in &buf {
        bytes.extend_from_slice(&[*r, *g, *b, *a]);
    }

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&bytes).map_err(|e| e.to_string())?;
    Ok((width, height))
}

fn rasterize_mesh(
    buf: &mut [[u8; 4]],
    width: u32,
    height: u32,
    clip: egui::Rect,
    mesh: &Mesh,
    font_image: &ColorImage,
) {
    let textured = matches!(mesh.texture_id, TextureId::Managed(0));
    let n = mesh.indices.len();
    for t in (0..n).step_by(3) {
        let v0 = &mesh.vertices[mesh.indices[t] as usize];
        let v1 = &mesh.vertices[mesh.indices[t + 1] as usize];
        let v2 = &mesh.vertices[mesh.indices[t + 2] as usize];

        // Bounding box, clipped to the scissor and the framebuffer.
        let tri_min_x = v0.pos.x.min(v1.pos.x).min(v2.pos.x).floor() as i32;
        let tri_max_x = v0.pos.x.max(v1.pos.x).max(v2.pos.x).ceil() as i32;
        let tri_min_y = v0.pos.y.min(v1.pos.y).min(v2.pos.y).floor() as i32;
        let tri_max_y = v0.pos.y.max(v1.pos.y).max(v2.pos.y).ceil() as i32;
        let x0 = tri_min_x
            .max(clip.min.x.floor() as i32)
            .max(0)
            .min(tri_max_x)
            .min(width as i32);
        let y0 = tri_min_y
            .max(clip.min.y.floor() as i32)
            .max(0)
            .min(tri_max_y)
            .min(height as i32);
        let x1 = tri_max_x
            .min(clip.max.x.ceil() as i32)
            .max(0)
            .min(width as i32);
        let y1 = tri_max_y
            .min(clip.max.y.ceil() as i32)
            .max(0)
            .min(height as i32);

        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5; // pixel center
                let py = y as f32 + 0.5;
                let Some(l) = bary(px, py, v0, v1, v2) else {
                    continue;
                };

                // Interpolate straight color and uv.
                let c = lerp3(v0.color, v1.color, v2.color, l);
                let mut src_a = f32::from(c[3]) / 255.0;
                if src_a <= 0.0 {
                    continue;
                }
                let (r, g, b) = if textured {
                    let u = lerp(v0.uv.x, v1.uv.x, v2.uv.x, l);
                    let v = lerp(v0.uv.y, v1.uv.y, v2.uv.y, l);
                    // White glyph: alpha channel is the coverage.
                    src_a *= f32::from(sample(font_image, u, v)[3]) / 255.0;
                    (f32::from(c[0]), f32::from(c[1]), f32::from(c[2]))
                } else {
                    (f32::from(c[0]), f32::from(c[1]), f32::from(c[2]))
                };

                // Straight-alpha "over" compositing.
                let idx = (y as usize) * width as usize + x as usize;
                let d = &mut buf[idx];
                let keep = 1.0 - src_a;
                d[0] = clamp8(f32::from(d[0]) * keep + r * src_a);
                d[1] = clamp8(f32::from(d[1]) * keep + g * src_a);
                d[2] = clamp8(f32::from(d[2]) * keep + b * src_a);
                d[3] = clamp8(f32::from(d[3]) * keep + src_a * 255.0);
            }
        }
    }
}

/// Barycentric coordinates of `p` in triangle (v0, v1, v2), or `None` if outside.
fn bary(px: f32, py: f32, v0: &Vertex, v1: &Vertex, v2: &Vertex) -> Option<(f32, f32, f32)> {
    let d = (v1.pos.y - v2.pos.y) * (v0.pos.x - v2.pos.x)
        + (v2.pos.x - v1.pos.x) * (v0.pos.y - v2.pos.y);
    if d.abs() < 1e-9 {
        return None;
    }
    let a = ((v1.pos.y - v2.pos.y) * (px - v2.pos.x) + (v2.pos.x - v1.pos.x) * (py - v2.pos.y)) / d;
    let b = ((v2.pos.y - v0.pos.y) * (px - v2.pos.x) + (v0.pos.x - v2.pos.x) * (py - v2.pos.y)) / d;
    let c = 1.0 - a - b;
    if a >= -1e-4 && b >= -1e-4 && c >= -1e-4 {
        Some((a, b, c))
    } else {
        None
    }
}

fn lerp(x: f32, y: f32, z: f32, l: (f32, f32, f32)) -> f32 {
    x * l.0 + y * l.1 + z * l.2
}

fn lerp3(c0: Color32, c1: Color32, c2: Color32, l: (f32, f32, f32)) -> [u8; 4] {
    [
        clamp8(lerp(
            f32::from(c0.r()),
            f32::from(c1.r()),
            f32::from(c2.r()),
            l,
        )),
        clamp8(lerp(
            f32::from(c0.g()),
            f32::from(c1.g()),
            f32::from(c2.g()),
            l,
        )),
        clamp8(lerp(
            f32::from(c0.b()),
            f32::from(c1.b()),
            f32::from(c2.b()),
            l,
        )),
        clamp8(lerp(
            f32::from(c0.a()),
            f32::from(c1.a()),
            f32::from(c2.a()),
            l,
        )),
    ]
}

fn sample(img: &ColorImage, u: f32, v: f32) -> [u8; 4] {
    let x = (u.clamp(0.0, 1.0) * img.width() as f32).min(img.width() as f32 - 1.0) as usize;
    let y = (v.clamp(0.0, 1.0) * img.height() as f32).min(img.height() as f32 - 1.0) as usize;
    let p = &img.pixels[y * img.width() + x];
    [p.r(), p.g(), p.b(), p.a()]
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
