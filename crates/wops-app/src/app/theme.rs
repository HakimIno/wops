use eframe::egui::{self, Color32, Stroke, Vec2};
use wops_core::ThemePreference;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub canvas: Color32,
    pub panel: Color32,
    pub panel_raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub danger: Color32,
}

impl Palette {
    pub(super) fn for_theme(theme: ThemePreference) -> Self {
        match theme {
            ThemePreference::Dark => Self {
                canvas: Color32::from_rgb(10, 12, 16),
                panel: Color32::from_rgb(15, 18, 23),
                panel_raised: Color32::from_rgb(20, 24, 31),
                border: Color32::from_rgb(39, 45, 56),
                text: Color32::from_rgb(226, 230, 237),
                muted: Color32::from_rgb(126, 136, 151),
                accent: Color32::from_rgb(47, 111, 237),
                accent_soft: Color32::from_rgb(22, 35, 54),
                success: Color32::from_rgb(52, 168, 112),
                danger: Color32::from_rgb(210, 67, 76),
            },
            ThemePreference::Light => Self {
                canvas: Color32::from_rgb(239, 242, 246),
                panel: Color32::from_rgb(249, 250, 252),
                panel_raised: Color32::WHITE,
                border: Color32::from_rgb(207, 214, 224),
                text: Color32::from_rgb(24, 30, 39),
                muted: Color32::from_rgb(97, 108, 123),
                accent: Color32::from_rgb(38, 91, 190),
                accent_soft: Color32::from_rgb(225, 233, 245),
                success: Color32::from_rgb(23, 155, 94),
                danger: Color32::from_rgb(218, 55, 81),
            },
        }
    }
}

pub(super) fn apply_theme(ctx: &egui::Context, theme: ThemePreference) {
    let palette = Palette::for_theme(theme);
    let mut visuals = match theme {
        ThemePreference::Dark => egui::Visuals::dark(),
        ThemePreference::Light => egui::Visuals::light(),
    };

    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.panel;
    visuals.extreme_bg_color = palette.canvas;
    visuals.faint_bg_color = palette.panel_raised;
    visuals.selection.bg_fill = palette.accent;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.bg_fill = palette.panel_raised;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.hovered.bg_fill = palette.accent_soft;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.active.bg_fill = palette.accent;
    ctx.set_visuals(visuals);

    let current_theme = ctx.theme();
    let mut style = (*ctx.style_of(current_theme)).clone();
    style.spacing.item_spacing = Vec2::splat(8.0);
    style.spacing.button_padding = Vec2::new(10.0, 7.0);
    style.visuals.window_corner_radius = 5.into();
    style.visuals.menu_corner_radius = 4.into();
    ctx.set_style_of(current_theme, style);
}
