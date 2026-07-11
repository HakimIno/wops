use std::{borrow::Cow, path::Path};

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use thiserror::Error;
use wgpu::util::DeviceExt;

use crate::{source::SourcePixels, transform::Transform2D};

pub const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SOURCE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ANIMATED_WIDTH: u32 = 192;
const ANIMATED_HEIGHT: u32 = 108;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSize {
    pub width: u32,
    pub height: u32,
}

impl CanvasSize {
    pub const FULL_HD: Self = Self {
        width: 1920,
        height: 1080,
    };

    pub fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::InvalidCanvasSize { width, height });
        }
        Ok(Self { width, height })
    }

    pub fn aspect_ratio(self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("canvas dimensions must be non-zero, got {width}x{height}")]
    InvalidCanvasSize { width: u32, height: u32 },
    #[error("could not load image source: {0}")]
    Image(#[from] image::ImageError),
    #[error("render layer {0} does not exist")]
    LayerNotFound(usize),
    #[error("RGBA frame has invalid stride or data length")]
    InvalidFrame,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayerUniform {
    model: [[f32; 4]; 4],
    opacity: f32,
    _padding: [f32; 3],
    uv_rect: [f32; 4],
}

enum SourceKind {
    Static,
    AnimatedGradient,
}

pub struct RenderLayer {
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    source_size: CanvasSize,
    source_kind: SourceKind,
    pub transform: Transform2D,
    pub opacity: f32,
    pub visible: bool,
}

impl RenderLayer {
    pub fn source_size(&self) -> CanvasSize {
        self.source_size
    }

    fn uniform(&self, canvas_size: CanvasSize) -> LayerUniform {
        LayerUniform {
            model: self
                .transform
                .model_matrix(canvas_size.width, canvas_size.height)
                .to_cols_array_2d(),
            opacity: self.opacity.clamp(0.0, 1.0),
            _padding: [0.0; 3],
            uv_rect: self.transform.crop.uv_rect(),
        }
    }
}

pub struct Compositor {
    canvas_size: CanvasSize,
    canvas_texture: wgpu::Texture,
    canvas_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    layers: Vec<RenderLayer>,
}

impl Compositor {
    pub fn new(device: &wgpu::Device, canvas_size: CanvasSize) -> Result<Self, RenderError> {
        let canvas_size = CanvasSize::new(canvas_size.width, canvas_size.height)?;
        let (canvas_texture, canvas_view) = create_canvas(device, canvas_size);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wops layer bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wops layer sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let pipeline = create_pipeline(device, &bind_group_layout);

        Ok(Self {
            canvas_size,
            canvas_texture,
            canvas_view,
            pipeline,
            bind_group_layout,
            sampler,
            layers: Vec::new(),
        })
    }

    pub fn canvas_size(&self) -> CanvasSize {
        self.canvas_size
    }

    pub fn canvas_view(&self) -> &wgpu::TextureView {
        &self.canvas_view
    }

    pub fn layers(&self) -> &[RenderLayer] {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut [RenderLayer] {
        &mut self.layers
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: CanvasSize) -> Result<(), RenderError> {
        let size = CanvasSize::new(size.width, size.height)?;
        if size == self.canvas_size {
            return Ok(());
        }
        let scale = Vec2::new(
            size.width as f32 / self.canvas_size.width as f32,
            size.height as f32 / self.canvas_size.height as f32,
        );
        for layer in &mut self.layers {
            layer.transform.position *= scale;
            layer.transform.size *= scale;
        }
        let (texture, view) = create_canvas(device, size);
        self.canvas_texture = texture;
        self.canvas_view = view;
        self.canvas_size = size;
        Ok(())
    }

    pub fn add_color_source(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: [u8; 4],
        transform: Transform2D,
    ) -> usize {
        self.add_pixels(
            device,
            queue,
            SourcePixels::color(color),
            transform,
            SourceKind::Static,
        )
    }

    pub fn add_smpte_source(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        transform: Transform2D,
    ) -> usize {
        self.add_pixels(
            device,
            queue,
            SourcePixels::smpte(560, 315),
            transform,
            SourceKind::Static,
        )
    }

    pub fn add_animated_source(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        transform: Transform2D,
    ) -> usize {
        self.add_pixels(
            device,
            queue,
            SourcePixels::animated_gradient(ANIMATED_WIDTH, ANIMATED_HEIGHT, 0.0),
            transform,
            SourceKind::AnimatedGradient,
        )
    }

    pub fn add_image_source(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: impl AsRef<Path>,
        transform: Transform2D,
    ) -> Result<usize, RenderError> {
        let pixels = SourcePixels::image(path)?;
        Ok(self.add_pixels(device, queue, pixels, transform, SourceKind::Static))
    }

    pub fn add_rgba_source(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
        stride: usize,
        transform: Transform2D,
    ) -> Result<usize, RenderError> {
        let pixels = packed_rgba(width, height, rgba, stride)?;
        Ok(self.add_pixels(
            device,
            queue,
            SourcePixels::rgba(width, height, pixels.into_owned()),
            transform,
            SourceKind::Static,
        ))
    }

    pub fn update_rgba_source(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layer_index: usize,
        width: u32,
        height: u32,
        rgba: &[u8],
        stride: usize,
    ) -> Result<(), RenderError> {
        let pixels = packed_rgba(width, height, rgba, stride)?;
        let layer = self
            .layers
            .get_mut(layer_index)
            .ok_or(RenderError::LayerNotFound(layer_index))?;
        let new_size = CanvasSize { width, height };
        if layer.source_size == new_size {
            write_texture_bytes(queue, &layer.texture, width, height, &pixels);
            return Ok(());
        }

        let texture = create_source_texture(device, new_size);
        write_texture_bytes(queue, &texture, width, height, &pixels);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = create_layer_bind_group(
            device,
            &self.bind_group_layout,
            &self.sampler,
            &view,
            &layer.uniform_buffer,
        );
        layer.texture = texture;
        layer._view = view;
        layer.bind_group = bind_group;
        layer.source_size = new_size;
        Ok(())
    }

    pub fn remove_layer(&mut self, layer_index: usize) -> Result<(), RenderError> {
        if layer_index >= self.layers.len() {
            return Err(RenderError::LayerNotFound(layer_index));
        }
        self.layers.remove(layer_index);
        Ok(())
    }

    pub fn update_animated_sources(&mut self, queue: &wgpu::Queue, elapsed_seconds: f32) {
        for layer in &mut self.layers {
            if matches!(layer.source_kind, SourceKind::AnimatedGradient) {
                let pixels = SourcePixels::animated_gradient(
                    layer.source_size.width,
                    layer.source_size.height,
                    elapsed_seconds,
                );
                write_texture(queue, &layer.texture, &pixels);
            }
        }
    }

    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for layer in &self.layers {
            queue.write_buffer(
                &layer.uniform_buffer,
                0,
                bytemuck::bytes_of(&layer.uniform(self.canvas_size)),
            );
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("wops compositor encoder"),
        });
        {
            let color_attachment = Some(wgpu::RenderPassColorAttachment {
                view: &self.canvas_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wops compositor pass"),
                color_attachments: &[color_attachment],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            for layer in &self.layers {
                if layer.visible && layer.opacity > 0.0 {
                    pass.set_bind_group(0, &layer.bind_group, &[]);
                    pass.draw(0..6, 0..1);
                }
            }
        }
        queue.submit([encoder.finish()]);
    }

    fn add_pixels(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixels: SourcePixels,
        transform: Transform2D,
        source_kind: SourceKind,
    ) -> usize {
        let size = CanvasSize {
            width: pixels.width,
            height: pixels.height,
        };
        let texture = create_source_texture(device, size);
        write_texture(queue, &texture, &pixels);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let uniform = LayerUniform {
            model: transform
                .model_matrix(self.canvas_size.width, self.canvas_size.height)
                .to_cols_array_2d(),
            opacity: 1.0,
            _padding: [0.0; 3],
            uv_rect: transform.crop.uv_rect(),
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wops layer uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = create_layer_bind_group(
            device,
            &self.bind_group_layout,
            &self.sampler,
            &view,
            &uniform_buffer,
        );
        self.layers.push(RenderLayer {
            texture,
            _view: view,
            bind_group,
            uniform_buffer,
            source_size: size,
            source_kind,
            transform,
            opacity: 1.0,
            visible: true,
        });
        self.layers.len() - 1
    }
}

fn create_layer_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wops layer bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    })
}

fn packed_rgba<'a>(
    width: u32,
    height: u32,
    rgba: &'a [u8],
    stride: usize,
) -> Result<Cow<'a, [u8]>, RenderError> {
    let row_len = width as usize * 4;
    let required = stride.saturating_mul(height as usize);
    if width == 0 || height == 0 || stride < row_len || rgba.len() < required {
        return Err(RenderError::InvalidFrame);
    }
    if stride == row_len {
        return Ok(Cow::Borrowed(&rgba[..row_len * height as usize]));
    }
    let mut packed = Vec::with_capacity(row_len * height as usize);
    for row in 0..height as usize {
        packed.extend_from_slice(&rgba[row * stride..row * stride + row_len]);
    }
    Ok(Cow::Owned(packed))
}

fn create_canvas(device: &wgpu::Device, size: CanvasSize) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("wops canvas"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: CANVAS_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_source_texture(device: &wgpu::Device, size: CanvasSize) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("wops source texture"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SOURCE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn write_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, pixels: &SourcePixels) {
    write_texture_bytes(queue, texture, pixels.width, pixels.height, &pixels.rgba);
}

fn write_texture_bytes(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    rgba: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn create_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wops compositor shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("compositor.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wops compositor pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wops compositor pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: CANVAS_FORMAT,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_sized_canvas() {
        assert!(matches!(
            CanvasSize::new(0, 1080),
            Err(RenderError::InvalidCanvasSize { .. })
        ));
    }

    #[test]
    fn full_hd_aspect_ratio_is_sixteen_by_nine() {
        assert!((CanvasSize::FULL_HD.aspect_ratio() - 16.0 / 9.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rgba_packing_removes_row_padding() {
        let input = [1, 2, 3, 4, 99, 99, 5, 6, 7, 8, 99, 99];
        let packed = packed_rgba(1, 2, &input, 6).unwrap();
        assert_eq!(&*packed, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn tightly_packed_rgba_is_borrowed() {
        let input = [1, 2, 3, 4];
        assert!(matches!(
            packed_rgba(1, 1, &input, 4).unwrap(),
            Cow::Borrowed(_)
        ));
    }
}
