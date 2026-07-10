use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use wops_core::Scene;

use super::{
    WopsApp,
    icons::{Icon, icon},
    theme::Palette,
    widgets::{empty_state, icon_action, separator, status_dot},
};

const DEFAULT_DOCK_HEIGHT: f32 = 248.0;
const MIN_DOCK_HEIGHT: f32 = 210.0;
const MAX_DOCK_HEIGHT: f32 = 320.0;

impl WopsApp {
    pub(super) fn dock(&mut self, root: &mut egui::Ui) {
        let p = self.palette();
        let panel = egui::Panel::bottom("dock")
            .default_size(DEFAULT_DOCK_HEIGHT)
            .min_size(MIN_DOCK_HEIGHT)
            .max_size(MAX_DOCK_HEIGHT)
            .resizable(true)
            .frame(egui::Frame::new().fill(p.panel))
            .show(root, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let dock_rect = ui.max_rect();

                ui.columns(3, |columns| {
                    self.scenes_section(&mut columns[0], p);
                    self.sources_section(&mut columns[1], p);
                    audio_section(&mut columns[2], p);
                });

                paint_column_dividers(ui, dock_rect, p);
            });

        root.painter().line_segment(
            [
                panel.response.rect.left_top(),
                panel.response.rect.right_top(),
            ],
            Stroke::new(1.0, p.border),
        );
    }

    fn scenes_section(&mut self, ui: &mut egui::Ui, p: Palette) {
        dock_section(ui, p, Icon::Layers, "Scenes", |ui| {
            let mut remove_scene = None;

            for (index, scene) in self.state.scenes.iter().enumerate() {
                let selected = index == self.selected_scene;
                let fill = if selected {
                    p.accent_soft
                } else {
                    Color32::TRANSPARENT
                };
                let response = egui::Frame::new()
                    .fill(fill)
                    .corner_radius(3)
                    .inner_margin(9)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            status_dot(ui, if selected { p.accent } else { p.muted });
                            ui.label(RichText::new(&scene.name).strong().color(p.text));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if selected {
                                        ui.label(RichText::new("ON AIR").size(9.0).color(p.accent));
                                    }
                                },
                            );
                        });
                    });

                if response.response.interact(egui::Sense::click()).clicked() {
                    self.selected_scene = index;
                }
                if response.response.secondary_clicked() && self.state.scenes.len() > 1 {
                    remove_scene = Some(index);
                }
            }

            if let Some(index) = remove_scene {
                self.state.scenes.remove(index);
                self.selected_scene = self.selected_scene.min(self.state.scenes.len() - 1);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if icon_action(ui, p, Icon::Add, "Add scene").clicked() {
                        let number = self.state.scenes.len() + 1;
                        self.state.scenes.push(Scene {
                            name: format!("Scene {number}"),
                        });
                        self.selected_scene = self.state.scenes.len() - 1;
                    }
                    ui.label(
                        RichText::new("Right-click to remove")
                            .size(10.0)
                            .color(p.muted),
                    );
                });
            });
        });
    }

    fn sources_section(&self, ui: &mut egui::Ui, p: Palette) {
        dock_section(ui, p, Icon::Video, "Sources", |ui| {
            if self.state.sources.is_empty() {
                empty_state(
                    ui,
                    p,
                    "No sources yet",
                    "Add a camera, display, or media file",
                );
            } else {
                for source in &self.state.sources {
                    ui.label(RichText::new(&source.name).color(p.text));
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                icon_action(ui, p, Icon::Add, "Add source")
                    .on_hover_text("Source capture arrives in phase 3");
            });
        });
    }
}

fn audio_section(ui: &mut egui::Ui, p: Palette) {
    dock_section(ui, p, Icon::Microphone, "Audio mixer", |ui| {
        audio_channel(ui, p, "Desktop Audio", "No device", 0.0);
        ui.add_space(10.0);
        audio_channel(ui, p, "Microphone", "No device", 0.0);
    });
}

fn dock_section(
    ui: &mut egui::Ui,
    p: Palette,
    section_icon: Icon,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::Frame::new()
        .fill(p.panel)
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 14,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::new(7.0, 8.0);
            ui.horizontal(|ui| {
                ui.add(icon(section_icon, 15.0));
                ui.label(RichText::new(title).strong().size(12.0).color(p.text));
            });
        });

    separator(ui, p);

    egui::Frame::new()
        .fill(p.panel)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());
            ui.spacing_mut().item_spacing = Vec2::splat(8.0);
            body(ui);
        });
}

fn audio_channel(ui: &mut egui::Ui, p: Palette, name: &str, device: &str, level: f32) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(name).strong().size(11.0).color(p.text));
            ui.label(RichText::new(device).size(10.0).color(p.muted));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("−∞ dB").size(10.0).color(p.muted));
        });
    });
    ui.add_space(4.0);
    ui.add(
        egui::ProgressBar::new(level)
            .fill(p.success)
            .desired_width(ui.available_width())
            .desired_height(5.0)
            .corner_radius(3),
    );
}

fn paint_column_dividers(ui: &egui::Ui, rect: egui::Rect, p: Palette) {
    let column_width = rect.width() / 3.0;
    for index in 1..3 {
        let x = rect.left() + column_width * index as f32;
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            Stroke::new(1.0, p.border),
        );
    }
}
