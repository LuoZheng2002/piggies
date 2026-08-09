


struct CameraUniform {
    sky_inverse_view_proj: mat4x4<f32>, // no translation
}

@group(0) @binding(0)
var<uniform> camera_uniform: CameraUniform;

@group(1) @binding(0)
var sky_texture: texture_cube<f32>;
@group(1) @binding(1)
var sky_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) direction: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var output: VertexOutput;
    // the hardcoded position
    let positions = array<vec2<f32>, 3>(
            vec2<f32>(-1.0, -1.0),
            vec2<f32>( 3.0, -1.0),
            vec2<f32>(-1.0,  3.0),
        );
    // this serves two purposes: one is to tell fragment shader where to draw the vertices on screen
    // the other is to find the world-space position for calculating the skybox pixel direction
    // the world-space position can have arbitrary distance to the camera, since the skybox pixel is the same given the direction is the same
    let clip_position = vec4<f32>(positions[vertex_index], 1.0, 1.0);
    output.clip_position = clip_position;
    output.direction = (sky_inverse_view_proj * clip_position).xyz;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // we do not need to normalize the direction
    let color = textureSample(sky_texture, sky_sampler, input.direction);
    return color;
}
