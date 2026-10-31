//! Live SVG preview rasterization (CR-004 / S-026): the display seam for the GUI's
//! right-side preview pane. Re-exports the exporters' raw-RGBA rasterizer so
//! `gui-app` never names the raster crates directly — ADR 009 seam discipline: the
//! GUI asks the core services, and they own the pipeline. The pane shows exactly what
//! FR5 exports because this is the same `svg_from_text` → resvg path as `to_png`.

pub use exporters::{RgbaImage, text_to_rgba};
