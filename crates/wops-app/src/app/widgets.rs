use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use super::{
    icons::{Icon, icon},
    theme::Palette,
};

pub(super) fn panel_heading(ui: &mut egui::Ui, p: Palette, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).strong().size(11.0).color(p.accent));
    ui.add_space(2.0);
    ui.label(RichText::new(subtitle).strong().size(15.0).color(p.text));
}

pub(super) fn separator(ui: &mut egui::Ui, p: Palette) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, p.border);
}

pub(super) fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

pub(super) fn info_row(ui: &mut egui::Ui, p: Palette, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(p.muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).strong().size(11.0).color(p.text));
        });
    });
}

pub(super) fn icon_action(
    ui: &mut egui::Ui,
    p: Palette,
    kind: Icon,
    label: &str,
) -> egui::Response {
    ui.add(
        egui::Button::image_and_text(
            icon(kind, 14.0),
            RichText::new(label).size(11.0).color(p.text),
        )
        .fill(p.panel_raised)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(3),
    )
}

pub(super) fn empty_state(ui: &mut egui::Ui, p: Palette, title: &str, subtitle: &str) {
    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).strong().size(12.0).color(p.text));
        ui.label(RichText::new(subtitle).size(10.0).color(p.muted));
    });
}

pub(super) fn fit_aspect(available: Vec2, aspect: f32) -> Vec2 {
    let width = available.x.max(1.0);
    let height = (width / aspect).min(available.y.max(1.0));
    Vec2::new((height * aspect).min(width), height)
}
