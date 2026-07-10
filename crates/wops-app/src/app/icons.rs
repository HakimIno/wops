use eframe::egui::{self, Vec2};

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Add,
    Layers,
    Microphone,
    Radio,
    Settings,
    Video,
}

pub(super) fn icon(kind: Icon, size: f32) -> egui::Image<'static> {
    let source = match kind {
        Icon::Add => egui::include_image!("../../assets/icons/circle-plus.svg"),
        Icon::Layers => egui::include_image!("../../assets/icons/layers.svg"),
        Icon::Microphone => egui::include_image!("../../assets/icons/mic.svg"),
        Icon::Radio => egui::include_image!("../../assets/icons/radio.svg"),
        Icon::Settings => egui::include_image!("../../assets/icons/settings.svg"),
        Icon::Video => egui::include_image!("../../assets/icons/video.svg"),
    };

    egui::Image::new(source).fit_to_exact_size(Vec2::splat(size))
}
