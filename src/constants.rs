//! Embedded WGSL for the Minecraft skin mesh.

/// Skin mesh shader (vertex + fragment).
///
/// Vertex stage: applies `uniforms.perspective`, `view`, `model`, and displaces vertices along
/// the normal by `uniforms.offset` (small positive values push the overlay layer outward).
///
/// Fragment stage: nearest-neighbor sampling via `s_skin`, discards near-transparent texels,
/// then two directional lights plus ambient on the shaded normal.
///
/// # Bind groups
///
/// - Group 0, binding 0: uniform buffer (`Uniforms`: projection, view, model, offset).
/// - Group 1, binding 0: skin `texture_2d`.
/// - Group 1, binding 1: sampler (configured as nearest in the render pipeline).
pub const SHADER: &str = r#"
struct Uniforms {
    perspective: mat4x4<f32>,
    view: mat4x4<f32>,
    model: mat4x4<f32>,
    offset: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_skin: texture_2d<f32>;
@group(1) @binding(1)
var s_skin: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let modelview = uniforms.view * uniforms.model;
    out.tex_coords = in.tex_coords;
    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz,
    );
    out.normal = normal_matrix * in.normal;
    let offset_position = in.position + in.normal * uniforms.offset;
    out.clip_position = uniforms.perspective * modelview * vec4<f32>(offset_position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_skin, s_skin, in.tex_coords);

    if (tex_color.a < 0.01) {
        discard;
    }

    let light_dir1 = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let light_dir2 = normalize(vec3<f32>(-1.0, 0.5, -0.5));

    let ambient = 0.5;
    let diff1 = max(dot(normalize(in.normal), light_dir1), 0.0);
    let diff2 = max(dot(normalize(in.normal), light_dir2), 0.0) * 0.3;

    let diffuse = (ambient + diff1 * 0.5 + diff2) * vec3<f32>(1.0, 1.0, 1.0);

    return vec4<f32>(tex_color.rgb * diffuse, tex_color.a);
}
"#;
