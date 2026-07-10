use eframe::egui::{self, RichText, Stroke, Vec2};

use super::{
    WopsApp,
    icons::{Icon, icon},
    widgets::status_dot,
};

const HEADER_HEIGHT: f32 = 38.0;

impl WopsApp {
    pub(super) fn header(&mut self, root: &mut egui::Ui) {
        let p = self.palette();
        let panel = egui::Panel::top("header")
            .exact_size(HEADER_HEIGHT)
            .frame(egui::Frame::new().fill(p.panel).inner_margin(7))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Settings").clicked() {
                                self.show_settings = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Exit WOPS").clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                        ui.menu_button("Edit", |ui| {
                            ui.add_enabled(false, egui::Button::new("Undo"));
                            ui.add_enabled(false, egui::Button::new("Redo"));
                        });
                        ui.menu_button("View", |ui| {
                            ui.checkbox(&mut self.studio_mode, "Studio mode");
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::image(icon(Icon::Settings, 14.0))
                                    .fill(p.panel_raised)
                                    .stroke(Stroke::new(1.0, p.border))
                                    .corner_radius(4)
                                    .min_size(Vec2::splat(30.0)),
                            )
                            .on_hover_text("Settings")
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("Ready").size(12.0).color(p.muted));
                        status_dot(ui, p.success);
                    });
                });
            });

        root.painter().line_segment(
            [
                panel.response.rect.left_bottom(),
                panel.response.rect.right_bottom(),
            ],
            Stroke::new(1.0, p.border),
        );
    }
}
