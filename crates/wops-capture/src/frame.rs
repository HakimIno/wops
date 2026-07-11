use std::{ops::Deref, time::Duration};

use crate::pool::FramePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba,
    Bgra,
    Rgbx,
    Bgrx,
    Yuyv,
    Nv12,
    Mjpeg,
}

#[derive(Debug)]
pub struct FrameBuffer {
    data: Option<Vec<u8>>,
    pool: FramePool,
}

impl FrameBuffer {
    pub(crate) fn new(data: Vec<u8>, pool: FramePool) -> Self {
        Self {
            data: Some(data),
            pool,
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.data.as_deref_mut().unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.data.as_ref().map_or(0, Vec::len)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Deref for FrameBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_deref().unwrap_or_default()
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            self.pool.recycle(data);
        }
    }
}

#[derive(Debug)]
pub struct VideoFrame {
    pub data: FrameBuffer,
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    /// Bytes between the first pixel of adjacent rows.
    pub stride: usize,
    pub timestamp: Duration,
}

impl VideoFrame {
    pub fn packed_len(&self) -> usize {
        self.stride.saturating_mul(self.height as usize)
    }
}
