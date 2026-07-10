use eframe::egui::{self, RichText, Stroke};

use super::{WopsApp, widgets::status_dot};

const STATUS_HEIGHT: f32 = 26.0;

impl WopsApp {
    pub(super) fn status_bar(&self, root: &mut egui::Ui) {
        let p = self.palette();
        let panel = egui::Panel::bottom("status_bar")
            .exact_size(STATUS_HEIGHT)
            .frame(egui::Frame::new().fill(p.panel).inner_margin(5))
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    status_dot(ui, p.success);
                    ui.label(RichText::new("Core connected").size(11.0).color(p.muted));
                    ui.separator();
                    ui.label(RichText::new("No dropped frames").size(11.0).color(p.muted));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:.0} FPS   •   CPU --   •   0 kb/s", self.fps))
                                .size(11.0)
                                .color(p.muted),
                        );
                    });
                });
            });

        root.painter().line_segment(
            [
                panel.response.rect.left_top(),
                panel.response.rect.right_top(),
            ],
            Stroke::new(1.0, p.border),
        );
    }
}
