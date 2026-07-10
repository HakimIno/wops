struct LayerUniform {
    model: mat4x4<f32>,
    opacity: f32,
    uv_rect: vec4<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> layer: LayerUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-0.5,  0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5, -0.5),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    var output: VertexOutput;
    output.position = layer.model * vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = mix(layer.uv_rect.xy, layer.uv_rect.zw, uvs[vertex_index]);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, input.uv);
    return color * layer.opacity;
}
