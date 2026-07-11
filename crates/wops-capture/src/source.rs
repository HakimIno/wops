use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use thiserror::Error;

use crate::VideoFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Screen,
    Window,
    Webcam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Starting,
    Active,
    Lost,
    Stopped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInfo {
    pub name: String,
    pub kind: CaptureKind,
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureEvent {
    State(CaptureState),
    Info(SourceInfo),
    RestoreToken(String),
    Error(String),
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture source is already running")]
    AlreadyRunning,
    #[error("capture backend failed: {0}")]
    Backend(String),
    #[error("capture thread failed to start: {0}")]
    Thread(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct FrameSink(Sender<VideoFrame>);

impl FrameSink {
    pub fn send(&self, frame: VideoFrame) {
        match self.0.try_send(frame) {
            Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

#[derive(Clone)]
pub struct EventSink(Sender<CaptureEvent>);

impl EventSink {
    pub fn send(&self, event: CaptureEvent) {
        let _ = self.0.send(event);
    }
}

pub struct CaptureChannels {
    pub frame_sink: FrameSink,
    pub frame_rx: Receiver<VideoFrame>,
    pub event_sink: EventSink,
    pub event_rx: Receiver<CaptureEvent>,
}

pub fn capture_channels() -> CaptureChannels {
    let (frame_tx, frame_rx) = bounded(3);
    let (event_tx, event_rx) = unbounded();
    CaptureChannels {
        frame_sink: FrameSink(frame_tx),
        frame_rx,
        event_sink: EventSink(event_tx),
        event_rx,
    }
}

pub trait VideoSource: Send {
    fn start(&mut self, frames: FrameSink, events: EventSink) -> Result<(), CaptureError>;
    fn stop(&mut self);
    fn info(&self) -> SourceInfo;
}
