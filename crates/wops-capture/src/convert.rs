use thiserror::Error;

use crate::{FramePool, PixelFormat, VideoFrame};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConvertError {
    #[error("conversion from {from:?} to {to:?} is not implemented")]
    Unsupported { from: PixelFormat, to: PixelFormat },
    #[error("source frame is shorter than its declared dimensions and stride")]
    Truncated,
}

pub fn convert(
    frame: &VideoFrame,
    to: PixelFormat,
    pool: &FramePool,
) -> Result<VideoFrame, ConvertError> {
    if frame.data.len() < frame.packed_len() {
        return Err(ConvertError::Truncated);
    }
    if frame.format == to {
        return copy_packed(frame, pool);
    }
    match (frame.format, to) {
        (PixelFormat::Bgra | PixelFormat::Bgrx, PixelFormat::Rgba) => bgra_to_rgba(frame, pool),
        (PixelFormat::Rgbx, PixelFormat::Rgba) => rgbx_to_rgba(frame, pool),
        (PixelFormat::Yuyv, PixelFormat::Rgba) => yuyv_to_rgba(frame, pool),
        (from, to) => Err(ConvertError::Unsupported { from, to }),
    }
}

fn output_frame(frame: &VideoFrame, pool: &FramePool) -> VideoFrame {
    VideoFrame {
        data: pool.acquire(frame.width as usize * frame.height as usize * 4),
        format: PixelFormat::Rgba,
        width: frame.width,
        height: frame.height,
        stride: frame.width as usize * 4,
        timestamp: frame.timestamp,
    }
}

fn copy_packed(frame: &VideoFrame, pool: &FramePool) -> Result<VideoFrame, ConvertError> {
    let bytes_per_pixel = match frame.format {
        PixelFormat::Rgba | PixelFormat::Bgra | PixelFormat::Rgbx | PixelFormat::Bgrx => 4,
        PixelFormat::Yuyv => 2,
        _ => {
            return Err(ConvertError::Unsupported {
                from: frame.format,
                to: frame.format,
            });
        }
    };
    let output_stride = frame.width as usize * bytes_per_pixel;
    let mut data = pool.acquire(output_stride * frame.height as usize);
    for row in 0..frame.height as usize {
        let source = &frame.data[row * frame.stride..row * frame.stride + output_stride];
        data.as_mut_slice()[row * output_stride..(row + 1) * output_stride].copy_from_slice(source);
    }
    Ok(VideoFrame {
        data,
        format: frame.format,
        width: frame.width,
        height: frame.height,
        stride: output_stride,
        timestamp: frame.timestamp,
    })
}

fn bgra_to_rgba(frame: &VideoFrame, pool: &FramePool) -> Result<VideoFrame, ConvertError> {
    let mut output = output_frame(frame, pool);
    for row in 0..frame.height as usize {
        let source = &frame.data[row * frame.stride..row * frame.stride + frame.width as usize * 4];
        let destination = &mut output.as_mut_row(row);
        for (source, destination) in source.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
            destination.copy_from_slice(&[source[2], source[1], source[0], 255]);
        }
    }
    Ok(output)
}

fn rgbx_to_rgba(frame: &VideoFrame, pool: &FramePool) -> Result<VideoFrame, ConvertError> {
    let mut output = output_frame(frame, pool);
    for row in 0..frame.height as usize {
        let source = &frame.data[row * frame.stride..row * frame.stride + frame.width as usize * 4];
        let destination = &mut output.as_mut_row(row);
        for (source, destination) in source.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
            destination.copy_from_slice(&[source[0], source[1], source[2], 255]);
        }
    }
    Ok(output)
}

fn yuyv_to_rgba(frame: &VideoFrame, pool: &FramePool) -> Result<VideoFrame, ConvertError> {
    let mut output = output_frame(frame, pool);
    for row in 0..frame.height as usize {
        let source = &frame.data[row * frame.stride..row * frame.stride + frame.width as usize * 2];
        let destination = &mut output.as_mut_row(row);
        for (source, destination) in source.chunks_exact(4).zip(destination.chunks_exact_mut(8)) {
            let y0 = source[0];
            let u = source[1];
            let y1 = source[2];
            let v = source[3];
            destination[..4].copy_from_slice(&yuv_to_rgba(y0, u, v));
            destination[4..].copy_from_slice(&yuv_to_rgba(y1, u, v));
        }
    }
    Ok(output)
}

fn yuv_to_rgba(y: u8, u: u8, v: u8) -> [u8; 4] {
    let c = i32::from(y).saturating_sub(16);
    let d = i32::from(u) - 128;
    let e = i32::from(v) - 128;
    let r = (298 * c + 409 * e + 128) >> 8;
    let g = (298 * c - 100 * d - 208 * e + 128) >> 8;
    let b = (298 * c + 516 * d + 128) >> 8;
    [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
        255,
    ]
}

impl VideoFrame {
    fn as_mut_row(&mut self, row: usize) -> &mut [u8] {
        &mut self.data.as_mut_slice()[row * self.stride..(row + 1) * self.stride]
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn frame(format: PixelFormat, data: &[u8], width: u32, stride: usize) -> VideoFrame {
        let pool = FramePool::new(1);
        let mut buffer = pool.acquire(data.len());
        buffer.as_mut_slice().copy_from_slice(data);
        VideoFrame {
            data: buffer,
            format,
            width,
            height: 1,
            stride,
            timestamp: Duration::ZERO,
        }
    }

    #[test]
    fn converts_bgra_with_padding() {
        let source = frame(PixelFormat::Bgra, &[30, 20, 10, 99, 0, 0, 0, 0], 1, 8);
        let converted = convert(&source, PixelFormat::Rgba, &FramePool::new(2)).unwrap();
        assert_eq!(&*converted.data, &[10, 20, 30, 255]);
    }

    #[test]
    fn converts_yuyv_pair() {
        let source = frame(PixelFormat::Yuyv, &[16, 128, 235, 128], 2, 4);
        let converted = convert(&source, PixelFormat::Rgba, &FramePool::new(2)).unwrap();
        assert_eq!(&converted.data[..4], &[0, 0, 0, 255]);
        assert_eq!(&converted.data[4..], &[255, 255, 255, 255]);
    }
}
