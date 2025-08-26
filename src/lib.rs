/*!
Minecraft 皮肤渲染器库

这个库提供了一个完整的 Minecraft 皮肤渲染解决方案，支持：
- 加载和渲染 Minecraft 皮肤
- 自定义姿势和视角
- 多种输出格式
- 离屏渲染

# 示例
*/
pub mod camera;
pub mod character;
pub mod constants;
pub mod model;
pub mod renderer;
pub mod texture;
pub mod utils;

/// 一键渲染 Minecraft 皮肤
///
/// # 参数
/// - filename: 输出图片文件名
/// - width: 图片宽度
/// - height: 图片高度
/// - texture: PNG 材质文件路径
/// - skin_type: 皮肤类型
/// - format: 输出图片格式（"png" 或 "webp"）
/// - yaw: 摄像机绕角色旋转角度
/// - pitch: 摄像机俯仰角度
/// - scale: 缩放比例
/// - head_yaw: 头部摇头角度
/// - head_pitch: 头部俯仰角度
/// - left_arm_roll: 左手侧举角度
/// - left_arm_pitch: 左手摆臂角度
/// - right_arm_roll: 右手侧举角度
/// - right_arm_pitch: 右手摆臂角度
/// - left_leg_pitch: 左腿抬腿角度
/// - right_leg_pitch: 右腿抬腿角度
///
/// # 返回
/// 渲染成功返回 Ok(())，否则返回错误
use pyo3::{
    pyfunction, pymodule,
    types::{PyModule, PyModuleMethods},
    wrap_pyfunction, Bound, PyResult,
};

use pyo3::types::PyBytes;
use pyo3::{Py, Python};
use std::io::Cursor;

#[pyfunction]
pub fn render_skin(
    py: Python<'_>,
    width: u32,
    height: u32,
    texture: &str,
    skin_type: &str,
    format: &str,
    yaw: f32,
    pitch: f32,
    scale: f32,
    head_yaw: f32,
    head_pitch: f32,
    left_arm_roll: f32,
    left_arm_pitch: f32,
    right_arm_roll: f32,
    right_arm_pitch: f32,
    left_leg_pitch: f32,
    right_leg_pitch: f32,
) -> PyResult<Py<PyBytes>> {
    use crate::camera::Camera;
    use crate::character::Character;
    use crate::renderer::{OutputFormat, Renderer};
    use image::DynamicImage;

    // 创建渲染器
    let renderer = Renderer::new();

    // 解析皮肤类型
    let skin_type = match skin_type.to_lowercase().as_str() {
        "default" | "classic" | "steve" => crate::character::SkinType::Classic,
        "slim" | "alex" => crate::character::SkinType::Slim,
        _ => crate::character::SkinType::Classic,
    };

    // 创建角色和相机
    let mut character = Character::new();
    character.skin_type = skin_type;
    let camera = Camera { yaw, pitch, scale };

    // 设置角色姿势
    character.posture.head_yaw = head_yaw;
    character.posture.head_pitch = head_pitch;
    character.posture.left_arm_roll = left_arm_roll;
    character.posture.left_arm_pitch = left_arm_pitch;
    character.posture.right_arm_roll = right_arm_roll;
    character.posture.right_arm_pitch = right_arm_pitch;
    character.posture.left_leg_pitch = left_leg_pitch;
    character.posture.right_leg_pitch = right_leg_pitch;

    // 设置皮肤文件
    character
        .load_skin_from_file(texture, renderer.get_display())
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to load skin: {e}"))
        })?;

    // 解析输出格式
    let output_format = match format.to_lowercase().as_str() {
        "png" => OutputFormat::Png,
        "webp" => OutputFormat::WebP,
        _ => OutputFormat::Png,
    };

    // 渲染到内存
    let img_buf = renderer
        .render(&character, &camera, width, height)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Render error: {e}")))?;

    let dyn_img = DynamicImage::ImageRgba8(img_buf);
    let mut buf = Vec::new();
    let img_format = match output_format {
        OutputFormat::Png => image::ImageFormat::Png,
        OutputFormat::WebP => image::ImageFormat::WebP,
    };
    dyn_img
        .write_to(&mut Cursor::new(&mut buf), img_format)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Encode error: {e}")))?;

    Ok(PyBytes::new(py, &buf).into())
}

#[pymodule]
fn eidolon(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(render_skin, m)?)?;
    Ok(())
}
