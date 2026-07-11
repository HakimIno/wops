//! V4L2 webcam capture.

use std::{
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_channel::{Sender, bounded};
use tracing::warn;
use v4l::{
    FourCC, buffer::Type, frameinterval::FrameIntervalEnum, framesize::FrameSizeEnum,
    io::traits::CaptureStream, prelude::MmapStream, video::Capture,
};

use crate::{
    CaptureError, CaptureEvent, CaptureKind, CaptureState, EventSink, FramePool, FrameSink,
    PixelFormat, SourceInfo, VideoFrame, VideoSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebcamDevice {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebcamMode {
    pub format: [u8; 4],
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, Clone)]
pub struct WebcamConfig {
    pub device: WebcamDevice,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

pub fn devices() -> Vec<WebcamDevice> {
    v4l::context::enum_devices()
        .into_iter()
        .map(|node| WebcamDevice {
            name: node
                .name()
                .unwrap_or_else(|| format!("Video device {}", node.index())),
            path: node.path().to_path_buf(),
        })
        .collect()
}

pub fn modes(path: impl AsRef<Path>) -> Result<Vec<WebcamMode>, CaptureError> {
    let device = v4l::Device::with_path(path).map_err(backend_error)?;
    let mut result = Vec::new();
    for description in device.enum_formats().map_err(backend_error)? {
        for size in device
            .enum_framesizes(description.fourcc)
            .map_err(backend_error)?
        {
            let sizes: Vec<_> = match size.size {
                FrameSizeEnum::Discrete(size) => vec![size],
                // Avoid expanding a potentially enormous stepwise mode range.
                FrameSizeEnum::Stepwise(size) => vec![v4l::framesize::Discrete {
                    width: size.max_width,
                    height: size.max_height,
                }],
            };
            for size in sizes {
                let intervals = device
                    .enum_frameintervals(description.fourcc, size.width, size.height)
                    .unwrap_or_default();
                if intervals.is_empty() {
                    result.push(WebcamMode {
                        format: description.fourcc.repr,
                        width: size.width,
                        height: size.height,
                        fps: 30,
                    });
                }
                for interval in intervals {
                    let fps = match interval.interval {
                        FrameIntervalEnum::Discrete(interval) if interval.numerator > 0 => {
                            interval.denominator / interval.numerator
                        }
                        FrameIntervalEnum::Stepwise(interval) if interval.min.numerator > 0 => {
                            interval.min.denominator / interval.min.numerator
                        }
                        _ => 30,
                    };
                    result.push(WebcamMode {
                        format: description.fourcc.repr,
                        width: size.width,
                        height: size.height,
                        fps: fps.max(1),
                    });
                }
            }
        }
    }
    result.sort_by_key(|mode| (mode.width, mode.height, mode.fps));
    result.dedup();
    Ok(result)
}

pub struct WebcamCapture {
    config: WebcamConfig,
    thread: Option<JoinHandle<()>>,
    stop_tx: Option<Sender<()>>,
}

impl WebcamCapture {
    pub fn new(config: WebcamConfig) -> Self {
        Self {
            config,
            thread: None,
            stop_tx: None,
        }
    }
}

impl VideoSource for WebcamCapture {
    fn start(&mut self, frames: FrameSink, events: EventSink) -> Result<(), CaptureError> {
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return Err(CaptureError::AlreadyRunning);
        }
        let config = self.config.clone();
        let (stop_tx, stop_rx) = bounded(1);
        let thread_events = events.clone();
        events.send(CaptureEvent::State(CaptureState::Starting));
        let handle = thread::Builder::new()
            .name("wops-webcam-capture".to_owned())
            .spawn(move || {
                if let Err(error) = run_webcam(&config, frames, thread_events.clone(), &stop_rx) {
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
            name: self.config.device.name.clone(),
            kind: CaptureKind::Webcam,
            width: self.config.width,
            height: self.config.height,
            fps_num: self.config.fps,
            fps_den: 1,
        }
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_webcam(
    config: &WebcamConfig,
    frames: FrameSink,
    events: EventSink,
    stop_rx: &crossbeam_channel::Receiver<()>,
) -> Result<(), String> {
    let device = v4l::Device::with_path(&config.device.path).map_err(|error| error.to_string())?;
    let supported = device.enum_formats().map_err(|error| error.to_string())?;
    let requested_fourcc = [FourCC::new(b"YUYV"), FourCC::new(b"MJPG")]
        .into_iter()
        .find(|fourcc| supported.iter().any(|format| format.fourcc == *fourcc))
        .ok_or_else(|| "webcam does not provide YUYV or MJPEG frames".to_owned())?;
    let format = device
        .set_format(&v4l::Format::new(
            config.width,
            config.height,
            requested_fourcc,
        ))
        .map_err(|error| error.to_string())?;
    let parameters = device
        .set_params(&v4l::video::capture::Parameters::with_fps(config.fps))
        .map_err(|error| error.to_string())?;
    let fps_num = parameters.interval.denominator;
    let fps_den = parameters.interval.numerator.max(1);
    events.send(CaptureEvent::Info(SourceInfo {
        name: config.device.name.clone(),
        kind: CaptureKind::Webcam,
        width: format.width,
        height: format.height,
        fps_num,
        fps_den,
    }));
    events.send(CaptureEvent::State(CaptureState::Active));

    let pool = FramePool::new(4);
    let mut stream = MmapStream::with_buffers(&device, Type::VideoCapture, 4)
        .map_err(|error| error.to_string())?;
    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        let (bytes, metadata) = stream.next().map_err(|error| error.to_string())?;
        let timestamp: Duration = metadata.timestamp.into();
        let frame = match format.fourcc.repr {
            [b'Y', b'U', b'Y', b'V'] => copy_yuyv(bytes, &format, timestamp, &pool),
            [b'M', b'J', b'P', b'G'] => decode_mjpeg(bytes, timestamp, &pool)?,
            other => {
                warn!(?other, "unsupported negotiated webcam format");
                return Err("webcam negotiated an unsupported pixel format".to_owned());
            }
        };
        frames.send(frame);
    }
    events.send(CaptureEvent::State(CaptureState::Stopped));
    Ok(())
}

fn copy_yuyv(
    bytes: &[u8],
    format: &v4l::Format,
    timestamp: Duration,
    pool: &FramePool,
) -> VideoFrame {
    let stride = if format.stride == 0 {
        format.width as usize * 2
    } else {
        format.stride as usize
    };
    let len = stride * format.height as usize;
    let mut data = pool.acquire(len);
    let copied = len.min(bytes.len());
    data.as_mut_slice()[..copied].copy_from_slice(&bytes[..copied]);
    VideoFrame {
        data,
        format: PixelFormat::Yuyv,
        width: format.width,
        height: format.height,
        stride,
        timestamp,
    }
}

fn decode_mjpeg(bytes: &[u8], timestamp: Duration, pool: &FramePool) -> Result<VideoFrame, String> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let pixels = image.into_raw();
    let mut data = pool.acquire(pixels.len());
    data.as_mut_slice().copy_from_slice(&pixels);
    Ok(VideoFrame {
        data,
        format: PixelFormat::Rgba,
        width,
        height,
        stride: width as usize * 4,
        timestamp,
    })
}

fn backend_error(error: std::io::Error) -> CaptureError {
    CaptureError::Backend(error.to_string())
}
