const PI: f32 = 3.14159265358979323846;

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec4<f32>, //4th component: 0 for directional light; 1 for point light
    color: vec4<f32>, // 4d for alignment purpose
}

struct LightUniform {
    light_count: u32,
    _pad0: vec3<u32>,
    lights: array<Light, 16>,
};

@group(2) @binding(0)
var<uniform> light_uniform: LightUniform;

struct PbrMaterial {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
}

@group(0) @binding(0)
var t_color: texture_2d<f32>;
@group(0) @binding(1)
var t_metallic: texture_2d<f32>;
@group(0) @binding(2)
var t_roughness: texture_2d<f32>;
@group(0) @binding(3)
var texture_sampler: sampler;
#group(0) @binding(4)
var<uniform> material: PbrMaterial;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.world_pos = (model_matrix * vec4<f32>(model.position, 1.0)).xyz;
    out.normal = (model_matrix * vec4<f32>(model.normal, 1.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.normal);
    let V = normalize(camera.view_pos - in.world_pos);
    for (var i: i32 = 0; i < light_uniform.light_count; i++) {
        let light = light_uniform.lights[i];
        let L_unnorm = select(light.position.xyz - in.world_pos, light.position.xyz, light.position.w == 0.0);
        let L = normalize(L_unnorm);
        let H = normalize(V + L);
    }
}

// GGX/Trowbridge-Reitz Normal Distribution Function
fn D(alpha: f32, N: vec3<f32>, H: vec3<f32>) -> f32{
    let numerator = pow(alpha, 2.0);
    let NdotH = max(dot(N, H), 0.0);
    let tmp = NdotH * NdotH * (alpha * alpha - 1.0) + 1.0;
    let denominator = PI * tmp * tmp;
    numerator / denominator
}

// Schlick-Beckmann Geometry Shadowing Function
fn G1(alpha: f32, N: vec3<f32>, X: vec3<f32>)-> f32{
    let numerator = max(dot(N, x), 0.0);
    let k = alpha / 2.0;
    let denominator = max(dot(N, x), 0.0) * (1.0 - k) + k;
    numerator / max(denominator, 0.000001)
}

// Smith Model
fn G(alpha: f32, N: vec3<f32>, V: vec3<f32>, L: vec3<f32>) -> f32 {
    G1(alpha, N, V) * G1(alpha, N, L)
}

// Fresnel-Schlick Function
fn F(F0: vec3<f32>, V: vec3<f32>, H: vec3<f32>) -> vec3<f32>{
    F0 + (vec3<f32>(1.0) - F0) * pow(1-max(dot(V, H), 0.0), 5.0)
}

fn PBR(F0: vec3<f32>, V: vec3<f32>, H: vec3<f32>, N: vec3<f32>, L: vec3<f32>, light_color: vec3<f32>) -> vec3<f32> {
    let albedo = material.base_color * textureSample(t_color, texture_sampler, in.tex_coords);
    let metallic = material.metallic * textureSample(t_metallic, texture_sampler, in.tex_coords);
    let roughness = material.roughness * textureSample(t_roughness, texture_sampler, in.tex_coords);

    let Ks = F(F0, V, H);
    let Kd = (vec3<f32>(1.0) - Ks) * (1.0 - metallic);

    let lambert: vec3<f32> = albedo / PI;
    let alpha = roughness * roughness;
    let cook_torrance_numerator = D(alpha, N, H) * G(alpha, N, V, L) * F(F0, V, H);
    let cook_torrance_denominator = 4.0 * max(dot(V, N), 0.0) * max(dot(L, N), 0.0);
    let cook_torrance = cook_torrance_numerator / max(cook_torrance_denominator, 0.000001);
    let BRDF = Kd * lambert + cook_torrance;
    let emissivity = 0.0; // can add later
    let outgoing_light = emissivity + BRDF * light_color * max(dot(L, N), 0.0);
    outgoing_light
}
