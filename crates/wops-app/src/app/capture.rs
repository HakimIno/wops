use crossbeam_channel::Receiver;
use glam::Vec2;
use tracing::{error, info};
use wops_capture::{
    CaptureEvent, CaptureState, FramePool, VideoFrame, VideoSource, WebcamDevice, capture_channels,
    convert,
    portal::PortalCapture,
    webcam::{WebcamCapture, WebcamConfig, WebcamMode},
};
use wops_core::{Source, SourceKind, SourceStatus};
use wops_render::{CanvasSize, Transform2D};

use super::WopsApp;

pub(super) struct ActiveCapture {
    pub source_id: u64,
    pub backend: Box<dyn VideoSource>,
    pub frame_rx: Receiver<VideoFrame>,
    pub event_rx: Receiver<CaptureEvent>,
    pub conversion_pool: FramePool,
    pub layer_index: Option<usize>,
}

impl WopsApp {
    pub(super) fn add_portal_source(&mut self, kind: SourceKind) {
        let restore_token = match kind {
            SourceKind::Screen => self.state.settings.screen_restore_token.clone(),
            SourceKind::Window => self.state.settings.window_restore_token.clone(),
            _ => return,
        };
        let backend: Box<dyn VideoSource> = match kind {
            SourceKind::Screen => Box::new(PortalCapture::screen(restore_token)),
            SourceKind::Window => Box::new(PortalCapture::window(restore_token)),
            _ => return,
        };
        self.start_capture(kind, backend);
    }

    pub(super) fn add_webcam_source(&mut self, device: WebcamDevice, mode: WebcamMode) {
        self.start_capture(
            SourceKind::Webcam,
            Box::new(WebcamCapture::new(WebcamConfig {
                device,
                width: mode.width,
                height: mode.height,
                fps: mode.fps.min(60),
            })),
        );
    }

    fn start_capture(&mut self, kind: SourceKind, mut backend: Box<dyn VideoSource>) {
        let channels = capture_channels();
        let source_id = self.next_source_id;
        self.next_source_id += 1;
        let info = backend.info();
        let source = Source {
            id: source_id,
            name: info.name,
            kind,
            status: SourceStatus::Starting,
            visible: true,
        };
        if let Err(capture_error) = backend.start(channels.frame_sink, channels.event_sink) {
            error!(%capture_error, "could not start capture source");
            self.state.sources.push(Source {
                status: SourceStatus::Error,
                ..source
            });
            return;
        }
        self.state.sources.push(source);
        self.captures.push(ActiveCapture {
            source_id,
            backend,
            frame_rx: channels.frame_rx,
            event_rx: channels.event_rx,
            conversion_pool: FramePool::new(3),
            layer_index: None,
        });
    }

    pub(super) fn poll_capture_sources(&mut self) {
        for index in 0..self.captures.len() {
            let events: Vec<_> = self.captures[index].event_rx.try_iter().collect();
            for event in events {
                self.apply_capture_event(index, event);
            }
            let latest_frame = self.captures[index].frame_rx.try_iter().last();
            if let Some(frame) = latest_frame {
                self.upload_capture_frame(index, frame);
            }
        }
    }

    fn apply_capture_event(&mut self, capture_index: usize, event: CaptureEvent) {
        let source_id = self.captures[capture_index].source_id;
        let Some(source_index) = self
            .state
            .sources
            .iter()
            .position(|source| source.id == source_id)
        else {
            return;
        };
        match event {
            CaptureEvent::State(state) => {
                self.state.sources[source_index].status = status_from_capture(state);
                let layer_visibility = match state {
                    CaptureState::Active => Some(self.state.sources[source_index].visible),
                    CaptureState::Lost | CaptureState::Stopped | CaptureState::Error => Some(false),
                    CaptureState::Starting => None,
                };
                if let (Some(visible), Some(layer_index), Some(compositor)) = (
                    layer_visibility,
                    self.captures[capture_index].layer_index,
                    &mut self.compositor,
                ) && let Some(layer) = compositor.layers_mut().get_mut(layer_index)
                {
                    layer.visible = visible;
                }
            }
            CaptureEvent::Info(source_info) => {
                let source = &mut self.state.sources[source_index];
                source.name = source_info.name;
                info!(
                    source_id,
                    width = source_info.width,
                    height = source_info.height,
                    fps_num = source_info.fps_num,
                    fps_den = source_info.fps_den,
                    "capture source active"
                );
            }
            CaptureEvent::RestoreToken(token) => {
                match self.state.sources[source_index].kind {
                    SourceKind::Screen => self.state.settings.screen_restore_token = Some(token),
                    SourceKind::Window => self.state.settings.window_restore_token = Some(token),
                    _ => {}
                }
                self.save_settings();
            }
            CaptureEvent::Error(message) => {
                self.state.sources[source_index].status = SourceStatus::Error;
                if let (Some(layer_index), Some(compositor)) = (
                    self.captures[capture_index].layer_index,
                    &mut self.compositor,
                ) && let Some(layer) = compositor.layers_mut().get_mut(layer_index)
                {
                    layer.visible = false;
                }
                error!(source_id, %message, "capture source error");
            }
        }
    }

    fn upload_capture_frame(&mut self, capture_index: usize, frame: VideoFrame) {
        let converted;
        let rgba = if frame.format == wops_capture::PixelFormat::Rgba {
            &frame
        } else {
            match convert(
                &frame,
                wops_capture::PixelFormat::Rgba,
                &self.captures[capture_index].conversion_pool,
            ) {
                Ok(frame) => {
                    converted = frame;
                    &converted
                }
                Err(convert_error) => {
                    error!(%convert_error, "could not convert capture frame");
                    return;
                }
            }
        };
        let (Some(compositor), Some(render_state)) = (&mut self.compositor, &self.render_state)
        else {
            return;
        };

        if let Some(layer_index) = self.captures[capture_index].layer_index {
            if let Err(render_error) = compositor.update_rgba_source(
                &render_state.device,
                &render_state.queue,
                layer_index,
                rgba.width,
                rgba.height,
                &rgba.data,
                rgba.stride,
            ) {
                error!(%render_error, "could not update capture texture");
            }
            return;
        }

        let source_id = self.captures[capture_index].source_id;
        let kind = self
            .state
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .map_or(SourceKind::Screen, |source| source.kind);
        let transform = source_transform(compositor.canvas_size(), kind);
        // Test layers are useful before capture starts; live sources replace them.
        for layer in compositor.layers_mut().iter_mut().take(3).skip(1) {
            layer.visible = false;
        }
        match compositor.add_rgba_source(
            &render_state.device,
            &render_state.queue,
            rgba.width,
            rgba.height,
            &rgba.data,
            rgba.stride,
            transform,
        ) {
            Ok(layer_index) => self.captures[capture_index].layer_index = Some(layer_index),
            Err(render_error) => error!(%render_error, "could not create capture texture"),
        }
    }

    pub(super) fn set_source_visibility(&mut self, source_id: u64, visible: bool) {
        if let Some(source) = self
            .state
            .sources
            .iter_mut()
            .find(|source| source.id == source_id)
        {
            source.visible = visible;
        }
        let Some(capture) = self
            .captures
            .iter()
            .find(|capture| capture.source_id == source_id)
        else {
            return;
        };
        if let (Some(layer_index), Some(compositor)) = (capture.layer_index, &mut self.compositor)
            && let Some(layer) = compositor.layers_mut().get_mut(layer_index)
        {
            layer.visible = visible;
        }
    }

    pub(super) fn remove_source(&mut self, source_id: u64) {
        self.state.sources.retain(|source| source.id != source_id);
        let Some(capture_index) = self
            .captures
            .iter()
            .position(|capture| capture.source_id == source_id)
        else {
            return;
        };
        let mut capture = self.captures.remove(capture_index);
        capture.backend.stop();
        let Some(removed_layer) = capture.layer_index else {
            return;
        };
        if let Some(compositor) = &mut self.compositor {
            if let Err(render_error) = compositor.remove_layer(removed_layer) {
                error!(%render_error, "could not remove capture layer");
            }
        }
        for capture in &mut self.captures {
            if let Some(layer_index) = &mut capture.layer_index
                && *layer_index > removed_layer
            {
                *layer_index -= 1;
            }
        }
    }
}

fn source_transform(canvas: CanvasSize, kind: SourceKind) -> Transform2D {
    let canvas_size = Vec2::new(canvas.width as f32, canvas.height as f32);
    match kind {
        SourceKind::Screen => Transform2D::new(canvas_size * 0.5, canvas_size),
        SourceKind::Window => Transform2D::new(canvas_size * 0.5, canvas_size * 0.82),
        SourceKind::Webcam => Transform2D::new(
            Vec2::new(canvas_size.x * 0.78, canvas_size.y * 0.76),
            canvas_size * Vec2::new(0.32, 0.32),
        ),
        SourceKind::Image | SourceKind::Color => {
            Transform2D::new(canvas_size * 0.5, canvas_size * 0.75)
        }
    }
}

fn status_from_capture(state: CaptureState) -> SourceStatus {
    match state {
        CaptureState::Starting => SourceStatus::Starting,
        CaptureState::Active => SourceStatus::Active,
        CaptureState::Lost => SourceStatus::Lost,
        CaptureState::Stopped => SourceStatus::Stopped,
        CaptureState::Error => SourceStatus::Error,
    }
}
