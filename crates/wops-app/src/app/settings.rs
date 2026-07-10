use eframe::egui::{self, RichText, Stroke};
use wops_core::ThemePreference;

use super::{
    WopsApp,
    theme::{Palette, apply_theme},
};

impl WopsApp {
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let p = self.palette();
        let mut open = self.show_settings;
        let mut changed_theme = None;
        egui::Window::new("Studio settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .frame(
                egui::Frame::window(&ctx.style_of(ctx.theme()))
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(5)
                    .inner_margin(18),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("APPEARANCE").strong().size(11.0).color(p.accent));
                ui.add_space(5.0);
                ui.label(
                    RichText::new("Make the workspace feel like yours.")
                        .size(12.0)
                        .color(p.muted),
                );
                ui.add_space(18.0);
                settings_grid(ui, &mut self.state.settings.theme, &mut changed_theme, p);
                ui.add_space(18.0);
                ui.label(
                    RichText::new(
                        "Stream, recording, and device settings will appear as those systems become available.",
                    )
                    .size(11.0)
                    .color(p.muted),
                );
            });
        self.show_settings = open;

        if let Some(theme) = changed_theme {
            apply_theme(ctx, theme);
            self.save_settings();
        }
    }
}

fn settings_grid(
    ui: &mut egui::Ui,
    selected_theme: &mut ThemePreference,
    changed_theme: &mut Option<ThemePreference>,
    p: Palette,
) {
    egui::Grid::new("general_settings")
        .num_columns(2)
        .spacing([32.0, 16.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Theme").color(p.text));
            egui::ComboBox::from_id_salt("theme")
                .selected_text(theme_label(*selected_theme))
                .show_ui(ui, |ui| {
                    for theme in [ThemePreference::Dark, ThemePreference::Light] {
                        if ui
                            .selectable_value(selected_theme, theme, theme_label(theme))
                            .changed()
                        {
                            *changed_theme = Some(theme);
                        }
                    }
                });
            ui.end_row();

            ui.label(RichText::new("Language").color(p.text));
            ui.label(RichText::new("English").color(p.muted));
            ui.end_row();
        });
}

fn theme_label(theme: ThemePreference) -> &'static str {
    match theme {
        ThemePreference::Dark => "Dark studio",
        ThemePreference::Light => "Light studio",
    }
}
