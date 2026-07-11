//! XDG Desktop Portal + PipeWire screen and window capture.

use std::{
    os::fd::OwnedFd,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
};
use crossbeam_channel::{Receiver, Sender, bounded};
use pipewire as pw;
use pw::{properties::properties, spa};
use tracing::{debug, warn};

use crate::{
    CaptureError, CaptureEvent, CaptureKind, CaptureState, EventSink, FramePool, FrameSink,
    PixelFormat, SourceInfo, VideoFrame, VideoSource,
};

struct PortalStream {
    node_id: u32,
    fd: OwnedFd,
    restore_token: Option<String>,
}

struct PipeWireData {
    format: spa::param::video::VideoInfoRaw,
    frames: FrameSink,
    events: EventSink,
    pool: FramePool,
    started_at: Instant,
    kind: CaptureKind,
}

pub struct PortalCapture {
    kind: CaptureKind,
    restore_token: Option<String>,
    thread: Option<JoinHandle<()>>,
    stop_tx: Option<Sender<()>>,
}

impl PortalCapture {
    pub fn screen(restore_token: Option<String>) -> Self {
        Self::new(CaptureKind::Screen, restore_token)
    }

    pub fn window(restore_token: Option<String>) -> Self {
        Self::new(CaptureKind::Window, restore_token)
    }

    fn new(kind: CaptureKind, restore_token: Option<String>) -> Self {
        Self {
            kind,
            restore_token,
            thread: None,
            stop_tx: None,
        }
    }
}

impl VideoSource for PortalCapture {
    fn start(&mut self, frames: FrameSink, events: EventSink) -> Result<(), CaptureError> {
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return Err(CaptureError::AlreadyRunning);
        }
        let (stop_tx, stop_rx) = bounded(1);
        let kind = self.kind;
        let restore_token = self.restore_token.clone();
        let thread_events = events.clone();
        events.send(CaptureEvent::State(CaptureState::Starting));
        let handle = thread::Builder::new()
            .name(format!("wops-{}-capture", kind_label(kind)))
            .spawn(move || {
                if let Err(error) =
                    run_capture(kind, restore_token, frames, thread_events.clone(), stop_rx)
                {
                    thread_events.send(CaptureEvent::State(CaptureState::Error));
                    thread_events.send(CaptureEvent::Error(error));
                }
            })?;
        self.stop_tx = Some(stop_tx);
        self.thread = Some(handle);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(stop) = self.stop_tx.take() {
            let _ = stop.try_send(());
        }
        self.thread.take();
    }

    fn info(&self) -> SourceInfo {
        SourceInfo {
            name: format!("{} Capture", kind_label(self.kind)),
            kind: self.kind,
            width: 0,
            height: 0,
            fps_num: 0,
            fps_den: 1,
        }
    }
}

impl Drop for PortalCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_capture(
    kind: CaptureKind,
    restore_token: Option<String>,
    frames: FrameSink,
    events: EventSink,
    stop_rx: Receiver<()>,
) -> Result<(), String> {
    let portal = futures_lite::future::block_on(open_portal(kind, restore_token.as_deref()))
        .map_err(|error| error.to_string())?;
    if let Some(token) = portal.restore_token {
        events.send(CaptureEvent::RestoreToken(token));
    }
    run_pipewire(kind, portal.node_id, portal.fd, frames, events, stop_rx)
        .map_err(|error| error.to_string())
}

async fn open_portal(
    kind: CaptureKind,
    restore_token: Option<&str>,
) -> ashpd::Result<PortalStream> {
    let proxy = Screencast::new().await?;
    let session = proxy.create_session(Default::default()).await?;
    let source_type = match kind {
        CaptureKind::Screen => SourceType::Monitor,
        CaptureKind::Window => SourceType::Window,
        CaptureKind::Webcam => unreachable!("webcams do not use the screencast portal"),
    };
    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(CursorMode::Embedded)
                .set_sources(Some(source_type.into()))
                .set_multiple(false)
                .set_restore_token(restore_token)
                .set_persist_mode(PersistMode::ExplicitlyRevoked),
        )
        .await?;
    let response = proxy
        .start(&session, None, Default::default())
        .await?
        .response()?;
    let Some(stream) = response.streams().first() else {
        return Err(ashpd::Error::NoResponse);
    };
    let node_id = stream.pipe_wire_node_id();
    let restore_token = response.restore_token().map(ToOwned::to_owned);
    let fd = proxy
        .open_pipe_wire_remote(&session, Default::default())
        .await?;
    Ok(PortalStream {
        node_id,
        fd,
        restore_token,
    })
}

fn run_pipewire(
    kind: CaptureKind,
    node_id: u32,
    fd: OwnedFd,
    frames: FrameSink,
    events: EventSink,
    stop_rx: Receiver<()>,
) -> Result<(), pw::Error> {
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_fd_rc(fd, None)?;
    let stream = pw::stream::StreamBox::new(
        &core,
        "wops-capture",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )?;

    let state_events = events.clone();
    let data = PipeWireData {
        format: Default::default(),
        frames,
        events: events.clone(),
        pool: FramePool::new(4),
        started_at: Instant::now(),
        kind,
    };
    let _listener = stream
        .add_local_listener_with_user_data(data)
        .state_changed(move |_, _, _, state| match state {
            pw::stream::StreamState::Streaming => {
                state_events.send(CaptureEvent::State(CaptureState::Active));
            }
            pw::stream::StreamState::Unconnected => {
                state_events.send(CaptureEvent::State(CaptureState::Lost));
            }
            pw::stream::StreamState::Error(message) => {
                state_events.send(CaptureEvent::State(CaptureState::Error));
                state_events.send(CaptureEvent::Error(message));
            }
            _ => {}
        })
        .param_changed(|_, data, id, param| {
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let Some(param) = param else {
                return;
            };
            if data.format.parse(param).is_err() {
                return;
            }
            let size = data.format.size();
            let framerate = data.format.framerate();
            data.events.send(CaptureEvent::Info(SourceInfo {
                name: format!("{} Capture", kind_label(data.kind)),
                kind: data.kind,
                width: size.width,
                height: size.height,
                fps_num: framerate.num,
                fps_den: framerate.denom.max(1),
            }));
            debug!(
                width = size.width,
                height = size.height,
                format = ?data.format.format(),
                "negotiated PipeWire video format"
            );
        })
        .process(|stream, data| {
            if let Some(mut buffer) = stream.dequeue_buffer() {
                copy_pipewire_frame(&mut buffer, data);
            }
        })
        .register()?;

    let mainloop_weak = mainloop.downgrade();
    let timer = mainloop.loop_().add_timer(move |_| {
        if stop_rx.try_recv().is_ok()
            && let Some(mainloop) = mainloop_weak.upgrade()
        {
            mainloop.quit();
        }
    });
    let interval = Duration::from_millis(100);
    timer.update_timer(Some(interval), Some(interval));

    let values = build_format_params();
    let mut params = [spa::pod::Pod::from_bytes(&values).expect("serialized format pod is valid")];
    stream.connect(
        spa::utils::Direction::Input,
        Some(node_id),
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;
    mainloop.run();
    events.send(CaptureEvent::State(CaptureState::Stopped));
    Ok(())
}

fn copy_pipewire_frame(buffer: &mut pw::buffer::Buffer<'_>, state: &mut PipeWireData) {
    let Some(data) = buffer.datas_mut().first_mut() else {
        return;
    };
    let chunk = data.chunk();
    let offset = chunk.offset() as usize;
    let signed_stride = chunk.stride();
    let size = state.format.size();
    let Some(format) = pixel_format(state.format.format()) else {
        warn!(format = ?state.format.format(), "unsupported PipeWire pixel format");
        return;
    };
    let stride = if signed_stride == 0 {
        size.width as usize * 4
    } else {
        signed_stride.unsigned_abs() as usize
    };
    let required = stride.saturating_mul(size.height as usize);
    let Some(source) = data.data() else {
        return;
    };
    if offset.saturating_add(required) > source.len() {
        return;
    }
    let mut destination = state.pool.acquire(required);
    for row in 0..size.height as usize {
        let source_row = if signed_stride < 0 {
            size.height as usize - 1 - row
        } else {
            row
        };
        let source_start = offset + source_row * stride;
        let destination_start = row * stride;
        destination.as_mut_slice()[destination_start..destination_start + stride]
            .copy_from_slice(&source[source_start..source_start + stride]);
    }
    state.frames.send(VideoFrame {
        data: destination,
        format,
        width: size.width,
        height: size.height,
        stride,
        timestamp: state.started_at.elapsed(),
    });
}

fn pixel_format(format: spa::param::video::VideoFormat) -> Option<PixelFormat> {
    match format {
        spa::param::video::VideoFormat::RGBA => Some(PixelFormat::Rgba),
        spa::param::video::VideoFormat::BGRA => Some(PixelFormat::Bgra),
        spa::param::video::VideoFormat::RGBx => Some(PixelFormat::Rgbx),
        spa::param::video::VideoFormat::BGRx => Some(PixelFormat::Bgrx),
        _ => None,
    }
}

fn build_format_params() -> Vec<u8> {
    let object = spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaType,
            Id,
            spa::param::format::MediaType::Video
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaSubtype,
            Id,
            spa::param::format::MediaSubtype::Raw
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            spa::param::video::VideoFormat::BGRx,
            spa::param::video::VideoFormat::BGRx,
            spa::param::video::VideoFormat::RGBx,
            spa::param::video::VideoFormat::BGRA,
            spa::param::video::VideoFormat::RGBA,
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            spa::utils::Rectangle {
                width: 4096,
                height: 4096
            }
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: 60, denom: 1 },
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 240, denom: 1 }
        ),
    );
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .expect("format pod serialization must succeed")
    .0
    .into_inner()
}

fn kind_label(kind: CaptureKind) -> &'static str {
    match kind {
        CaptureKind::Screen => "Screen",
        CaptureKind::Window => "Window",
        CaptureKind::Webcam => "Webcam",
    }
}
