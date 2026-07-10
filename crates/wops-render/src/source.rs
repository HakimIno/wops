use std::path::Path;

use image::ImageError;

pub(crate) struct SourcePixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl SourcePixels {
    pub fn color(color: [u8; 4]) -> Self {
        let mut rgba = color.to_vec();
        premultiply_alpha(&mut rgba);
        Self {
            width: 1,
            height: 1,
            rgba,
        }
    }

    pub fn smpte(width: u32, height: u32) -> Self {
        const BARS: [[u8; 4]; 7] = [
            [191, 191, 191, 255],
            [191, 191, 0, 255],
            [0, 191, 191, 255],
            [0, 191, 0, 255],
            [191, 0, 191, 255],
            [191, 0, 0, 255],
            [0, 0, 191, 255],
        ];
        let mut rgba = vec![0; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let bar = ((x as usize * BARS.len()) / width as usize).min(BARS.len() - 1);
                let mut color = BARS[bar];
                if y > height * 3 / 4 {
                    let checker = ((x / 24) + (y / 24)).is_multiple_of(2);
                    color = if checker {
                        [24, 27, 34, 255]
                    } else {
                        [225, 228, 234, 255]
                    };
                }
                let offset = ((y * width + x) * 4) as usize;
                rgba[offset..offset + 4].copy_from_slice(&color);
            }
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn animated_gradient(width: u32, height: u32, time_seconds: f32) -> Self {
        let mut rgba = vec![0; (width * height * 4) as usize];
        let shift = time_seconds * 1.4;
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;
                let wave = ((u * 8.0 + shift).sin() * 0.5 + 0.5) * 255.0;
                let offset = ((y * width + x) * 4) as usize;
                rgba[offset] = wave as u8;
                rgba[offset + 1] = ((v + shift * 0.08).fract() * 210.0 + 35.0) as u8;
                rgba[offset + 2] = (255.0 - wave * 0.55) as u8;
                rgba[offset + 3] = 255;
            }
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn image(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let image = image::open(path)?.into_rgba8();
        let (width, height) = image.dimensions();
        let mut rgba = image.into_raw();
        premultiply_alpha(&mut rgba);
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}

fn premultiply_alpha(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        pixel[0] = ((u16::from(pixel[0]) * alpha + 127) / 255) as u8;
        pixel[1] = ((u16::from(pixel[1]) * alpha + 127) / 255) as u8;
        pixel[2] = ((u16::from(pixel[2]) * alpha + 127) / 255) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_source_is_premultiplied() {
        assert_eq!(
            SourcePixels::color([200, 100, 50, 128]).rgba,
            [100, 50, 25, 128]
        );
    }

    #[test]
    fn smpte_source_has_requested_dimensions() {
        let source = SourcePixels::smpte(70, 40);
        assert_eq!(source.rgba.len(), 70 * 40 * 4);
    }
}
