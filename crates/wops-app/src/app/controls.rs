use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use super::{
    WopsApp,
    icons::{Icon, icon},
    theme::Palette,
    widgets::{info_row, panel_heading, separator},
};

const CONTROL_WIDTH: f32 = 248.0;

impl WopsApp {
    pub(super) fn controls(&mut self, root: &mut egui::Ui) {
        let p = self.palette();
        let panel = egui::Panel::right("controls")
            .exact_size(CONTROL_WIDTH)
            .frame(egui::Frame::new().fill(p.panel).inner_margin(14))
            .show(root, |ui| {
                panel_heading(ui, p, "OUTPUT", "Broadcast controls");
                ui.add_space(16.0);

                let live = ui.add(
                    egui::Button::image_and_text(
                        icon(Icon::Radio, 15.0),
                        RichText::new("GO LIVE").strong().color(Color32::WHITE),
                    )
                    .fill(p.accent)
                    .corner_radius(4)
                    .min_size(Vec2::new(ui.available_width(), 42.0)),
                );
                if live.clicked() {
                    self.show_settings = true;
                }
                live.on_hover_text("Configure a streaming destination first");

                ui.add_space(8.0);
                ui.add(
                    egui::Button::image_and_text(
                        icon(Icon::Video, 15.0),
                        RichText::new("START RECORDING").strong().color(p.danger),
                    )
                    .fill(p.panel_raised)
                    .stroke(Stroke::new(1.0, p.danger))
                    .corner_radius(4)
                    .min_size(Vec2::new(ui.available_width(), 40.0)),
                )
                .on_hover_text("Recording setup is coming in a later phase");

                ui.add_space(20.0);
                separator(ui, p);
                ui.add_space(16.0);
                ui.label(
                    RichText::new("QUICK SETTINGS")
                        .strong()
                        .size(11.0)
                        .color(p.muted),
                );
                ui.add_space(10.0);
                info_row(ui, p, "Resolution", "1920 × 1080");
                info_row(ui, p, "Frame rate", "60 FPS");
                info_row(ui, p, "Encoder", "Auto");
                info_row(ui, p, "Audio", "48 kHz");

                ui.add_space(18.0);
                studio_mode_card(ui, p, &mut self.studio_mode);
            });

        root.painter().line_segment(
            [
                panel.response.rect.left_top(),
                panel.response.rect.left_bottom(),
            ],
            Stroke::new(1.0, p.border),
        );
    }
}

fn studio_mode_card(ui: &mut egui::Ui, p: Palette, studio_mode: &mut bool) {
    egui::Frame::new()
        .fill(p.panel_raised)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(4)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Studio mode")
                    .strong()
                    .size(12.0)
                    .color(p.text),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("Preview changes before sending them live.")
                    .size(11.0)
                    .color(p.muted),
            );
            ui.add_space(7.0);
            ui.toggle_value(studio_mode, "Enable studio mode");
        });
}
