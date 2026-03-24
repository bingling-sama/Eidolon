//! 着色器常量模块
//!
//! 这个模块包含了用于渲染 Minecraft 皮肤的 WGSL 着色器代码。
//! 着色器支持纹理映射、光照计算和透明度处理。

/// WGSL 着色器
///
/// 包含顶点着色器和片段着色器，处理：
/// - 顶点变换（模型、视图、投影矩阵）
/// - 法线偏移（用于外层皮肤膨胀）
/// - 纹理采样（最近邻过滤）
/// - 双光源漫反射光照
/// - 透明像素丢弃
///
/// # Bind Groups
///
/// - Group 0, Binding 0: Uniforms (perspective, view, model, offset)
/// - Group 1, Binding 0: 皮肤纹理 (texture_2d)
/// - Group 1, Binding 1: 纹理采样器 (sampler)
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
