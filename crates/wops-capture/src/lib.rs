//! Linux video capture sources and frame transport.

mod convert;
mod frame;
mod pool;
mod source;

pub mod portal;
pub mod webcam;

pub use convert::{ConvertError, convert};
pub use frame::{FrameBuffer, PixelFormat, VideoFrame};
pub use pool::FramePool;
pub use source::{
    CaptureError, CaptureEvent, CaptureKind, CaptureState, EventSink, FrameSink, SourceInfo,
    VideoSource, capture_channels,
};
pub use webcam::{WebcamConfig, WebcamDevice, WebcamMode};
