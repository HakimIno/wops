use eframe::egui::{self, Color32, RichText};
use std::time::Duration;
use tracing::{debug, error};
use wops_core::{AppState, CoreChannels, Event, Settings, ThemePreference, core_channels};

const PANEL_HEIGHT: f32 = 230.0;
const STATUS_HEIGHT: f32 = 26.0;

pub struct WopsApp {
    state: AppState,
    channels: CoreChannels,
    show_settings: bool,
    fps: f32,
}

impl WopsApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        apply_theme(&creation_context.egui_ctx, settings.theme);
        debug!(renderer = ?creation_context.wgpu_render_state.is_some(), "created UI");

        Self {
            state: AppState {
                settings,
                ..AppState::default()
            },
            channels: core_channels(),
            show_settings: false,
            fps: 0.0,
        }
    }

    fn poll_core_events(&mut self) {
        for event in self.channels.event_rx.try_iter() {
            match event {
                Event::FrameStats(stats) => self.fps = stats.fps,
                Event::Error(message) => error!(%message, "core error"),
                other => debug!(?other, "core event"),
            }
        }
    }

    fn menu_bar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("menu_bar")
            .min_size(28.0)
            .max_size(28.0)
            .show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Settings...").clicked() {
                            self.show_settings = true;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("Edit", |_| {});
                    ui.menu_button("View", |_| {});
                    ui.menu_button("Tools", |_| {});
                    ui.menu_button("Help", |_| {});
                });
            });
    }

    fn status_bar(&self, root: &mut egui::Ui) {
        egui::Panel::bottom("status_bar")
            .min_size(STATUS_HEIGHT)
            .max_size(STATUS_HEIGHT)
            .show(root, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(format!("CPU: --  |  FPS: {:.1}", self.fps));
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(118, 45, 45), "REC  ●");
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(45, 100, 70), "LIVE  ●");
                });
            });
    }

    fn dock(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("dock")
            .default_size(PANEL_HEIGHT)
            .min_size(160.0)
            .max_size(400.0)
            .resizable(true)
            .show(root, |ui| {
                ui.columns(4, |columns| {
                    section(&mut columns[0], "Scenes", |ui| {
                        for scene in &self.state.scenes {
                            let _ = ui.selectable_label(true, &scene.name);
                        }
                        placeholder_buttons(ui);
                    });
                    section(&mut columns[1], "Sources", |ui| {
                        ui.weak("No sources");
                        placeholder_buttons(ui);
                    });
                    section(&mut columns[2], "Audio Mixer", |ui| {
                        ui.weak("No audio devices");
                        ui.add_enabled(false, egui::Slider::new(&mut 0.0, 0.0..=1.0));
                    });
                    section(&mut columns[3], "Controls", |ui| {
                        ui.add_enabled(false, egui::Button::new("Start Streaming"));
                        ui.add_enabled(false, egui::Button::new("Start Recording"));
                        if ui.button("Settings").clicked() {
                            self.show_settings = true;
                        }
                    });
                });
            });
    }

    fn preview(&self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Preview");
                ui.add_space(8.0);
                let available = ui.available_size();
                let response = ui.allocate_response(available, egui::Sense::hover());
                ui.painter().rect_filled(response.rect, 3.0, Color32::BLACK);
                ui.painter().text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Preview will appear here",
                    egui::FontId::proportional(18.0),
                    Color32::from_gray(125),
                );
            });
        });
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut open = self.show_settings;
        let mut changed_theme = None;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.heading("General");
                ui.separator();
                egui::Grid::new("general_settings")
                    .num_columns(2)
                    .spacing([24.0, 12.0])
                    .show(ui, |ui| {
                        ui.label("Theme");
                        egui::ComboBox::from_id_salt("theme")
                            .selected_text(match self.state.settings.theme {
                                ThemePreference::Dark => "Dark",
                                ThemePreference::Light => "Light",
                            })
                            .show_ui(ui, |ui| {
                                for (theme, label) in [
                                    (ThemePreference::Dark, "Dark"),
                                    (ThemePreference::Light, "Light"),
                                ] {
                                    if ui
                                        .selectable_value(
                                            &mut self.state.settings.theme,
                                            theme,
                                            label,
                                        )
                                        .changed()
                                    {
                                        changed_theme = Some(theme);
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Language");
                        ui.add_enabled(
                            false,
                            egui::TextEdit::singleline(&mut self.state.settings.language),
                        );
                        ui.end_row();
                    });
                ui.add_space(18.0);
                ui.weak("More settings will be added in later phases.");
            });
        self.show_settings = open;

        if let Some(theme) = changed_theme {
            apply_theme(ctx, theme);
            self.save_settings();
        }
    }

    fn save_settings(&self) {
        if let Err(error) = self.state.settings.save() {
            error!(%error, "could not save settings");
        }
    }
}

impl eframe::App for WopsApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_core_events();
        let stable_dt = ctx.input(|input| input.stable_dt).max(f32::EPSILON);
        self.fps = 1.0 / stable_dt;
        if let Some(inner_rect) = ctx.input(|input| input.viewport().inner_rect) {
            self.state.settings.window_width = inner_rect.width();
            self.state.settings.window_height = inner_rect.height();
        }
        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / 60.0));
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        self.menu_bar(root);
        self.status_bar(root);
        self.dock(root);
        self.preview(root);
        self.settings_window(&ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_settings();
    }
}

fn apply_theme(ctx: &egui::Context, theme: ThemePreference) {
    match theme {
        ThemePreference::Dark => ctx.set_visuals(egui::Visuals::dark()),
        ThemePreference::Light => ctx.set_visuals(egui::Visuals::light()),
    }
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_height(PANEL_HEIGHT - 20.0);
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong());
        ui.separator();
        body(ui);
    });
}

fn placeholder_buttons(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
            let _ = ui.small_button("+");
            let _ = ui.small_button("−");
        });
    });
}
