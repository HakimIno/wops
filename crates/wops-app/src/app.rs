mod capture;
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
use std::time::{Duration, Instant};
use theme::{Palette, apply_theme};
use tracing::{debug, error, info, warn};
use wops_core::{AppState, CoreChannels, Event, Settings, core_channels};
use wops_render::{CanvasSize, Compositor, Transform2D};

use capture::ActiveCapture;

pub struct WopsApp {
    state: AppState,
    channels: CoreChannels,
    show_settings: bool,
    fps: f32,
    selected_scene: usize,
    studio_mode: bool,
    compositor: Option<Compositor>,
    render_state: Option<eframe::egui_wgpu::RenderState>,
    preview_texture: Option<egui::TextureId>,
    render_started_at: Instant,
    last_render_at: Instant,
    captures: Vec<ActiveCapture>,
    next_source_id: u64,
}

impl WopsApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, mut settings: Settings) -> Self {
        egui_extras::install_image_loaders(&creation_context.egui_ctx);
        apply_theme(&creation_context.egui_ctx, settings.theme);
        debug!(renderer = ?creation_context.wgpu_render_state.is_some(), "created UI");

        if CanvasSize::new(settings.canvas_width, settings.canvas_height).is_err() {
            settings.canvas_width = CanvasSize::FULL_HD.width;
            settings.canvas_height = CanvasSize::FULL_HD.height;
        }

        let render_state = creation_context.wgpu_render_state.clone();
        let (compositor, preview_texture) = if let Some(render_state) = &render_state {
            let adapter = render_state.adapter.get_info();
            info!(
                name = %adapter.name,
                backend = ?adapter.backend,
                device_type = ?adapter.device_type,
                "initialized preview GPU"
            );
            let (compositor, texture) = create_compositor(render_state, &settings);
            (Some(compositor), Some(texture))
        } else {
            (None, None)
        };
        if render_state.is_none() {
            warn!("wgpu render state is unavailable; preview compositor is disabled");
        }

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
            compositor,
            render_state,
            preview_texture,
            render_started_at: Instant::now(),
            last_render_at: Instant::now(),
            captures: Vec::new(),
            next_source_id: 1,
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

    fn resize_canvas(&mut self, size: CanvasSize) {
        let (Some(compositor), Some(render_state), Some(texture_id)) = (
            &mut self.compositor,
            &self.render_state,
            self.preview_texture,
        ) else {
            return;
        };
        if let Err(error) = compositor.resize(&render_state.device, size) {
            error!(%error, "could not resize canvas");
            return;
        }
        render_state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                &render_state.device,
                compositor.canvas_view(),
                eframe::wgpu::FilterMode::Linear,
                texture_id,
            );
        self.state.settings.canvas_width = size.width;
        self.state.settings.canvas_height = size.height;
        self.save_settings();
    }
}

impl eframe::App for WopsApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_core_events();
        self.poll_capture_sources();
        let frame_interval = Duration::from_secs_f32(1.0 / 60.0);
        let elapsed = self.last_render_at.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
        let now = Instant::now();
        self.fps = 1.0 / now.duration_since(self.last_render_at).as_secs_f32();
        self.last_render_at = now;

        if let Some(inner_rect) = ctx.input(|input| input.viewport().inner_rect) {
            self.state.settings.window_width = inner_rect.width();
            self.state.settings.window_height = inner_rect.height();
        }

        if let (Some(compositor), Some(render_state)) = (&mut self.compositor, &self.render_state) {
            compositor.update_animated_sources(
                &render_state.queue,
                self.render_started_at.elapsed().as_secs_f32(),
            );
            compositor.render(&render_state.device, &render_state.queue);
        }

        // Phase 6 moves this fixed-rate pacing to a dedicated render thread.
        ctx.request_repaint();
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

fn create_compositor(
    render_state: &eframe::egui_wgpu::RenderState,
    settings: &Settings,
) -> (Compositor, egui::TextureId) {
    let canvas_size = CanvasSize::new(settings.canvas_width, settings.canvas_height)
        .unwrap_or(CanvasSize::FULL_HD);
    let mut compositor = Compositor::new(&render_state.device, canvas_size)
        .expect("the fallback canvas dimensions are valid");
    let center = glam::Vec2::new(
        canvas_size.width as f32 / 2.0,
        canvas_size.height as f32 / 2.0,
    );
    let canvas = glam::Vec2::new(canvas_size.width as f32, canvas_size.height as f32);

    compositor.add_color_source(
        &render_state.device,
        &render_state.queue,
        [8, 12, 19, 255],
        Transform2D::new(center, canvas),
    );
    let pattern = compositor.add_smpte_source(
        &render_state.device,
        &render_state.queue,
        Transform2D::new(center, canvas * 0.82),
    );
    compositor.layers_mut()[pattern].opacity = 0.88;
    let animated = compositor.add_animated_source(
        &render_state.device,
        &render_state.queue,
        Transform2D::new(
            glam::Vec2::new(canvas.x * 0.73, canvas.y * 0.72),
            canvas * glam::Vec2::new(0.34, 0.34),
        ),
    );
    compositor.layers_mut()[animated].transform.rotation_radians = -0.08;
    compositor.layers_mut()[animated].opacity = 0.94;
    compositor.render(&render_state.device, &render_state.queue);

    let texture_id = render_state.renderer.write().register_native_texture(
        &render_state.device,
        compositor.canvas_view(),
        eframe::wgpu::FilterMode::Linear,
    );
    (compositor, texture_id)
}
