mod controls;
mod dock;
mod header;
mod icons;
mod preview;
mod settings;
mod status_bar;
mod theme;
mod widgets;

use eframe::egui;
use std::time::Duration;
use theme::{Palette, apply_theme};
use tracing::{debug, error};
use wops_core::{AppState, CoreChannels, Event, Settings, core_channels};

pub struct WopsApp {
    state: AppState,
    channels: CoreChannels,
    show_settings: bool,
    fps: f32,
    selected_scene: usize,
    studio_mode: bool,
}

impl WopsApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        egui_extras::install_image_loaders(&creation_context.egui_ctx);
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
            selected_scene: 0,
            studio_mode: false,
        }
    }

    fn palette(&self) -> Palette {
        Palette::for_theme(self.state.settings.theme)
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
        self.header(root);
        self.status_bar(root);
        self.controls(root);
        self.dock(root);
        self.preview(root);
        self.settings_window(&ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_settings();
    }
}
