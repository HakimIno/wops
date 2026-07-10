use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use super::{WopsApp, widgets::fit_aspect};

impl WopsApp {
    pub(super) fn preview(&self, root: &mut egui::Ui) {
        let p = self.palette();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.canvas).inner_margin(18))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("PROGRAM").strong().size(11.0).color(p.accent));
                        ui.label(
                            RichText::new(
                                self.state
                                    .scenes
                                    .get(self.selected_scene)
                                    .map_or("Scene", |scene| &scene.name),
                            )
                            .strong()
                            .size(17.0)
                            .color(p.text),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Fit  •  100%").size(11.0).color(p.muted));
                    });
                });
                ui.add_space(12.0);

                let aspect_ratio = self.compositor.as_ref().map_or(16.0 / 9.0, |compositor| {
                    compositor.canvas_size().aspect_ratio()
                });
                let preview_size = fit_aspect(ui.available_size(), aspect_ratio);
                ui.vertical_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 3.0, Color32::from_rgb(3, 5, 8));
                    if let Some(texture_id) = self.preview_texture {
                        ui.painter().image(
                            texture_id,
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.painter().rect_stroke(
                        rect,
                        3.0,
                        Stroke::new(1.0, p.border),
                        egui::StrokeKind::Inside,
                    );

                    if self.preview_texture.is_none() {
                        let center = rect.center();
                        ui.painter().text(
                            center - Vec2::new(0.0, 8.0),
                            egui::Align2::CENTER_CENTER,
                            "GPU preview unavailable",
                            egui::FontId::proportional(14.0),
                            p.text,
                        );
                        ui.painter().text(
                            center + Vec2::new(0.0, 14.0),
                            egui::Align2::CENTER_CENTER,
                            "WOPS requires the wgpu renderer",
                            egui::FontId::proportional(11.0),
                            p.muted,
                        );
                    }
                });
            });
    }
}
