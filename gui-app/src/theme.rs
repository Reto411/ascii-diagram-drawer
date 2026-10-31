//! Material Design theme layer (FR14 / [ADR 017](../../doc/adr/017-material-design-title-bar.md)).
//!
//! One module owns the design tokens and turns them into an egui [`Style`]:
//! the M3 color roles, 4dp-based spacing, corner radii and elevation shadows.
//! Both palettes are installed on the context via `set_style_of`, so the
//! existing FR13 Light/Dark/Native selection (which drives egui's resolved
//! theme) picks the right Material palette — no third visual mode is
//! introduced (ADR 017, decision 1).
//!
//! Scope per CR-006: static styling only — colors, typography fallback,
//! elevation. No animated ripples (egui has no animation system). The canvas
//! is *not* themed: it paints its own fixed surface and monospace glyphs
//! (WYSIWYG invariant, ADR 017 decision 3).

use crate::config::Theme;
use eframe::egui;
use egui::epaint::FontColorTransferFunction;
use egui::{Color32, CornerRadius, Shadow, Stroke, Style, Visuals};

/// The Material Design 3 baseline color roles for one palette.
///
/// Values are the M3 baseline scheme (the "Material" purple), which is what
/// "styled according to Material Design" means without a brand override:
/// light per `surface #FEF7FF` / `primary #6750A4`, dark per
/// `surface #141218` / `primary #D0BCFF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub surface: Color32,
    pub on_surface: Color32,
    pub surface_variant: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub error: Color32,
}

/// The light Material palette (M3 baseline).
pub const LIGHT: Palette = Palette {
    primary: Color32::from_rgb(0x67, 0x50, 0xA4),
    on_primary: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    primary_container: Color32::from_rgb(0xEA, 0xDD, 0xFF),
    on_primary_container: Color32::from_rgb(0x21, 0x00, 0x5D),
    surface: Color32::from_rgb(0xFE, 0xF7, 0xFF),
    on_surface: Color32::from_rgb(0x1D, 0x1B, 0x20),
    surface_variant: Color32::from_rgb(0xE7, 0xE0, 0xEC),
    on_surface_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
    outline: Color32::from_rgb(0x79, 0x74, 0x7E),
    outline_variant: Color32::from_rgb(0xCA, 0xC4, 0xD0),
    error: Color32::from_rgb(0xB3, 0x26, 0x1E),
};

/// The dark Material palette (M3 baseline).
pub const DARK: Palette = Palette {
    primary: Color32::from_rgb(0xD0, 0xBC, 0xFF),
    on_primary: Color32::from_rgb(0x38, 0x1E, 0x72),
    primary_container: Color32::from_rgb(0x4F, 0x37, 0x8B),
    on_primary_container: Color32::from_rgb(0xEA, 0xDD, 0xFF),
    surface: Color32::from_rgb(0x14, 0x12, 0x18),
    on_surface: Color32::from_rgb(0xE6, 0xE0, 0xE9),
    surface_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
    on_surface_variant: Color32::from_rgb(0xCA, 0xC4, 0xD0),
    outline: Color32::from_rgb(0x93, 0x8F, 0x99),
    outline_variant: Color32::from_rgb(0x49, 0x45, 0x4F),
    error: Color32::from_rgb(0xF2, 0xB8, 0xB5),
};

/// Blends `a` toward `b` by fraction `t` (0..=1) — used for M3 "state layers"
/// (hover = on-surface at ~8% over the surface, pressed at ~16%).
#[must_use]
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t).round() as u8,
    )
}

/// The elevation shadow for floating surfaces (menus, dialogs, the app bar's
/// bottom edge): M3 elevation 1–2, expressed as egui's single soft shadow.
#[must_use]
pub fn elevation(dark: bool) -> Shadow {
    Shadow {
        offset: [0, 1],
        blur: 8,
        spread: 0,
        color: if dark {
            Color32::from_black_alpha(96)
        } else {
            Color32::from_black_alpha(48)
        },
    }
}

/// Installs both Material palettes on `ctx` and activates the one matching
/// `theme`'s preference — call this **before the first UI pass** (eframe's
/// `CreationContext`, or the headless harness) so the very first frame is already
/// Material: egui snapshots the active style into each [`egui::Ui`] when it is
/// created, so styles installed mid-pass only take effect from the next frame
/// and the layout would settle over two frames (button rows grow by 12px when
/// `interact_size` changes), which breaks first-frame geometry assumptions.
pub fn install(ctx: &egui::Context, theme: Theme) {
    ctx.set_style_of(egui::Theme::Light, material_style(false));
    ctx.set_style_of(egui::Theme::Dark, material_style(true));
    ctx.set_theme(theme.preference());
}

/// Builds the egui [`Style`] for the given palette (ADR 017, decision 1).
///
/// Typography: eframe's bundled proportional font stands in for Roboto
/// ("best effort" per the ADR — pixel-exact Material typography is not
/// guaranteed across OSes); fonts are left untouched here, which also keeps
/// the canvas monospace family exactly what the rest of the app expects.
#[must_use]
pub fn material_style(dark: bool) -> Style {
    let p = if dark { DARK } else { LIGHT };
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    // Font-atlas alpha curve: egui's `Visuals::dark()`/`light()` defaults carry a
    // THEME-DEPENDENT glyph transfer function (raw coverage in light mode,
    // 2c-c² in dark mode), so the *same* glyph anti-aliases differently per
    // palette. The canvas is WYSIWYG and must be pixel-identical under both
    // palettes (ADR 017 decision 3), so pin egui's crate-default curve in both —
    // it is also the sharper one, which suits the fixed light-on-dark canvas.
    visuals.text_options.color_transfer_function = FontColorTransferFunction::default();

    // Surfaces: panels (app bar, menu bar, sidebars, status bar) and windows.
    visuals.panel_fill = p.surface;
    visuals.window_fill = p.surface;

    // Non-interactive chrome text/icons sit on the surface.
    visuals.widgets.noninteractive.fg_stroke = egui::epaint::Stroke::new(1.0, p.on_surface);
    visuals.widgets.noninteractive.bg_fill = p.surface;

    // Buttons and other weak widgets: surface-variant fill, outline stroke.
    let mut inactive = visuals.widgets.inactive;
    inactive.weak_bg_fill = p.surface_variant;
    inactive.fg_stroke = egui::epaint::Stroke::new(1.0, p.on_surface);
    inactive.bg_stroke = egui::epaint::Stroke::new(1.0, p.outline);
    visuals.widgets.inactive = inactive;

    // M3 state layers: hover ~8%, pressed ~16% on-surface over the surface.
    let mut hovered = visuals.widgets.hovered;
    hovered.weak_bg_fill = blend(p.surface_variant, p.on_surface, 0.08);
    hovered.fg_stroke = egui::epaint::Stroke::new(1.0, p.on_surface);
    hovered.bg_stroke = egui::epaint::Stroke::new(1.0, p.outline);
    visuals.widgets.hovered = hovered;

    let mut active = visuals.widgets.active;
    active.weak_bg_fill = blend(p.surface_variant, p.on_surface, 0.16);
    active.fg_stroke = egui::epaint::Stroke::new(1.0, p.on_surface);
    active.bg_stroke = egui::epaint::Stroke::new(1.0, p.outline);
    visuals.widgets.active = active;

    // Selection and links use the primary role.
    visuals.selection.bg_fill = p.primary_container;
    visuals.selection.stroke = Stroke::new(1.0, p.on_primary_container);
    visuals.hyperlink_color = p.primary;

    // Elevation: soft shadow on floating surfaces (menus/dialogs/windows).
    let shadow = elevation(dark);
    visuals.window_shadow = shadow;
    visuals.popup_shadow = shadow;

    // Corner radii: M3 small (4dp) for widgets, medium (12dp) for windows.
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);
    visuals.window_corner_radius = CornerRadius::same(12);

    // Spacing on the 4dp grid. Vertical item spacing stays at 4 pt — M3's
    // comfortable 8 pt would grow the dense template-editor window by ~150 pt
    // and push its Save button off a 768-pt-tall screen. `interact_size` keeps
    // egui's default row height (20 pt): the layout was tuned to it, and M3's
    // 48 dp touch targets are for fingers, not a dense desktop tool.
    let style = Style {
        spacing: egui::Spacing {
            item_spacing: egui::vec2(8.0, 4.0),
            ..Default::default()
        },
        visuals,
        ..Default::default()
    };
    // Fonts are deliberately untouched: in egui 0.37 they live in the context
    // memory, not the style — and the canvas monospace family must stay exactly
    // what `draw_canvas` measures against (AC5).
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_tokens_are_m3_baseline() {
        assert_eq!(LIGHT.primary, Color32::from_rgb(0x67, 0x50, 0xA4));
        assert_eq!(LIGHT.on_primary, Color32::WHITE);
        assert_eq!(LIGHT.surface, Color32::from_rgb(0xFE, 0xF7, 0xFF));
        assert_eq!(LIGHT.on_surface, Color32::from_rgb(0x1D, 0x1B, 0x20));
        assert_eq!(LIGHT.outline_variant, Color32::from_rgb(0xCA, 0xC4, 0xD0));
    }

    #[test]
    fn dark_tokens_are_m3_baseline() {
        assert_eq!(DARK.primary, Color32::from_rgb(0xD0, 0xBC, 0xFF));
        assert_eq!(DARK.surface, Color32::from_rgb(0x14, 0x12, 0x18));
        assert_eq!(DARK.on_surface, Color32::from_rgb(0xE6, 0xE0, 0xE9));
        assert_eq!(DARK.error, Color32::from_rgb(0xF2, 0xB8, 0xB5));
    }

    #[test]
    fn palettes_are_distinct() {
        assert_ne!(LIGHT.surface, DARK.surface);
        assert_ne!(LIGHT.primary, DARK.primary);
    }

    #[test]
    fn blend_is_end_point_exact_and_monotone() {
        assert_eq!(blend(LIGHT.surface, LIGHT.on_surface, 0.0), LIGHT.surface);
        assert_eq!(
            blend(LIGHT.surface, LIGHT.on_surface, 1.0),
            LIGHT.on_surface
        );
        let mid = blend(LIGHT.surface, LIGHT.on_surface, 0.5);
        // Light surface is near-white, on-surface near-black: the midpoint must
        // sit strictly between them in luminance.
        let lum = |c: Color32| (c.r() as f32 + c.g() as f32 + c.b() as f32) / 3.0;
        assert!(lum(LIGHT.surface) > lum(mid));
        assert!(lum(mid) > lum(LIGHT.on_surface));
    }

    #[test]
    fn style_uses_palette_roles() {
        for (dark, p) in [(false, LIGHT), (true, DARK)] {
            let style = material_style(dark);
            assert_eq!(style.visuals.panel_fill, p.surface);
            assert_eq!(style.visuals.window_fill, p.surface);
            assert_eq!(style.visuals.selection.bg_fill, p.primary_container);
            assert_eq!(style.visuals.hyperlink_color, p.primary);
            let fg = style.visuals.widgets.noninteractive.fg_stroke.color;
            assert_eq!(fg, p.on_surface);
            assert_eq!(
                style.visuals.widgets.inactive.weak_bg_fill,
                p.surface_variant
            );
        }
    }

    #[test]
    fn spacing_is_on_the_4dp_grid() {
        let style = material_style(false);
        for v in [style.spacing.item_spacing.x, style.spacing.item_spacing.y] {
            assert_eq!(v % 4.0, 0.0, "spacing value {v} is not on the 4dp grid");
        }
        // Row height stays at egui's default — see the note in `material_style`.
        assert_eq!(
            style.spacing.interact_size,
            egui::Spacing::default().interact_size
        );
    }

    #[test]
    fn corner_radii_and_elevation_are_set() {
        for dark in [false, true] {
            let style = material_style(dark);
            assert_eq!(style.visuals.window_corner_radius, CornerRadius::same(12));
            assert_eq!(
                style.visuals.widgets.inactive.corner_radius,
                CornerRadius::same(4)
            );
            let shadow = style.visuals.window_shadow;
            assert!(shadow.blur > 0);
            assert_ne!(shadow.color, Color32::TRANSPARENT);
        }
    }

    #[test]
    fn construction_is_deterministic() {
        // Whole-`Style` equality is not usable in egui 0.37: `NumberFormatter`
        // compares its internal `Arc<dyn Fn>` by pointer, so two independently
        // constructed styles never compare equal. Compare the parts this module
        // actually controls instead.
        for dark in [false, true] {
            let (a, b) = (material_style(dark), material_style(dark));
            assert_eq!(a.visuals, b.visuals);
            assert_eq!(a.spacing, b.spacing);
            assert_eq!(a.interaction, b.interaction);
        }
        let (light, dark) = (material_style(false), material_style(true));
        assert_ne!(light.visuals, dark.visuals);
    }
}
