use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Default for Crop {
    fn default() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

impl Crop {
    pub(crate) fn uv_rect(self) -> [f32; 4] {
        let left = self.left.clamp(0.0, 1.0);
        let top = self.top.clamp(0.0, 1.0);
        [
            left,
            top,
            (1.0 - self.right).clamp(left, 1.0),
            (1.0 - self.bottom).clamp(top, 1.0),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    /// Center position in canvas pixels.
    pub position: Vec2,
    /// Displayed size in canvas pixels.
    pub size: Vec2,
    pub rotation_radians: f32,
    pub crop: Crop,
}

impl Transform2D {
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            position,
            size,
            rotation_radians: 0.0,
            crop: Crop::default(),
        }
    }

    pub(crate) fn model_matrix(self, canvas_width: u32, canvas_height: u32) -> Mat4 {
        let canvas = Vec2::new(canvas_width as f32, canvas_height as f32);
        let translation = Vec3::new(
            self.position.x * 2.0 / canvas.x - 1.0,
            1.0 - self.position.y * 2.0 / canvas.y,
            0.0,
        );
        let scale = Vec3::new(
            self.size.x * 2.0 / canvas.x,
            self.size.y * 2.0 / canvas.y,
            1.0,
        );

        Mat4::from_translation(translation)
            * Mat4::from_rotation_z(-self.rotation_radians)
            * Mat4::from_scale(scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_center_maps_to_clip_center() {
        let transform = Transform2D::new(Vec2::new(960.0, 540.0), Vec2::new(1920.0, 1080.0));
        let mapped = transform
            .model_matrix(1920, 1080)
            .transform_point3(Vec3::ZERO);
        assert!(mapped.abs().max_element() < f32::EPSILON);
    }

    #[test]
    fn crop_is_clamped_to_valid_uv_space() {
        let crop = Crop {
            left: -1.0,
            top: 0.25,
            right: 0.2,
            bottom: 2.0,
        };
        assert_eq!(crop.uv_rect(), [0.0, 0.25, 0.8, 0.25]);
    }
}
