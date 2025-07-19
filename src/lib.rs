//! Minecraft 皮肤渲染器库
//!
//! 这个库提供了一个完整的 Minecraft 皮肤渲染解决方案，支持：
//! - 加载和渲染 Minecraft 皮肤
//! - 自定义姿势和视角
//! - 多种输出格式
//! - 离屏渲染
//!
//! # 示例
//!
//! ```rust
//! use skinviewer::Renderer;
//!
//! let mut renderer = Renderer::new();
//! renderer.load_skin_from_file("path/to/skin.png");
//! renderer.render_to_image("output.png", (800, 600));
//! ```

pub mod constants;
pub mod model;
pub mod texture;
pub mod utils;

use glium::backend::glutin::headless::Headless;
use glium::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use glium::index::{NoIndices, PrimitiveType};
use glium::uniforms::SamplerWrapFunction::Repeat;
use glium::{uniform, BackfaceCullingMode, DepthTest, DrawParameters, Program, Surface, Texture2d};
use glutin::platform::unix::HeadlessContextExt;
use glutin::ContextBuilder;
use image::{ImageBuffer, ImageFormat, Rgba};

use crate::constants::{FRAGMENT_SHADER, VERTEX_SHADER};
use crate::model::Model;
use crate::texture::Texture;
use crate::utils::view_matrix;

/// Minecraft 皮肤类型
#[derive(Debug, Clone, Copy)]
pub enum SkinType {
    /// 默认皮肤类型（Steve 样式）
    Default,
    /// 细手臂皮肤类型（Alex 样式）
    Slim,
}

/// 输出图片格式
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// PNG 格式
    Png,
    /// WebP 格式
    WebP,
}

/// Minecraft 角色姿势
///
/// 定义了角色的各个身体部位的旋转角度
#[derive(Debug, Clone, Copy)]
pub struct Posture {
    /// 角色头部摇头角度（XZ 平面绕 Y 轴旋转），0~180，90 是正前，0 是正左，180 是正右
    pub head_yaw: f32,
    /// 角色头部俯仰角度（YZ 平面绕 X 轴旋转），0~180，90 是正前，0 是垂直向下看，180 是垂直向上看
    pub head_pitch: f32,
    /// 左手侧举角度（XY 平面绕 Z 轴旋转），0~180，90 是向右侧平举，0 是垂直向下，180 是垂直向上抬起
    pub left_arm_roll: f32,
    /// 左手摆臂角度（YZ 平面绕 X 轴旋转），0~360，0 是垂直向下，90 是水平前摆，180 是垂直向上，270 是水平向后
    pub left_arm_pitch: f32,
    /// 右手侧举角度（XY 平面绕 Z 轴旋转），0~180，90 是向右侧平举，0 是垂直向下，180 是垂直向上抬起
    pub right_arm_roll: f32,
    /// 右手摆臂角度（YZ 平面绕 X 轴旋转），0~360，0 是垂直向下，90 是水平前摆，180 是垂直向上，270 是水平向后
    pub right_arm_pitch: f32,
    /// 左腿抬腿角度（YZ 平面绕 X 轴旋转），0~180，90 是垂直于地面，0 是水平前摆，180 是水平后摆
    pub left_leg_pitch: f32,
    /// 右腿抬腿角度（YZ 平面绕 X 轴旋转），0~180，90 是垂直于地面，0 是水平前摆，180 是水平后摆
    pub right_leg_pitch: f32,
}

/// 预定义的默认姿势
pub struct DefaultPostures;

impl DefaultPostures {
    /// 站立姿势 - 所有角度为 0
    pub const STAND: Posture = Posture {
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 0.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 0.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
}

/// Minecraft 皮肤渲染器
///
/// 这是库的主要结构体，提供了完整的皮肤渲染功能。
/// 支持加载皮肤、设置姿势、调整视角、渲染图片等功能。
///
/// # 示例
///
/// ```rust
/// use skinviewer::Renderer;
///
/// let mut renderer = Renderer::new();
/// renderer.load_skin_from_file("skin.png")?;
/// renderer.set_posture(DefaultPostures::STAND);
/// renderer.render_to_image("output.png", (800, 600));
/// ```
pub struct Renderer {
    /// OpenGL 显示上下文
    display: Headless,
    /// 着色器程序
    program: Program,
    /// 绘制参数
    params: DrawParameters<'static>,
    /// 皮肤纹理，可选是因为可以只渲染披风
    pub skin: Option<Texture>,
    /// 皮肤类型，如果给了皮肤还是 None 的话就自动判断皮肤类型
    pub skin_type: SkinType,
    /// 3D 模型
    model: Model,
    /// 披风文件，可选是因为可以只渲染皮肤
    pub cape: Option<Vec<u8>>,
    /// 名称标签，None 就是不加 NameTag
    pub nametag: Option<String>,
    /// 当前姿势
    pub posture: Posture,
    /// 摄像机视角绕角色旋转角度（XZ 平面绕 Y 轴旋转），0~360，0 是正前，90 是正右，180 是正后，270 是正左
    pub yaw: f32,
    /// 摄像机视角绕角色俯仰角度（YZ 平面绕 X 轴旋转），0~180，90 是正前，0 是脚下，180 是头顶
    pub pitch: f32,
    /// 缩放比例，0~1
    pub scale: f32,
}

impl Renderer {
    /// 创建新的渲染器实例
    ///
    /// 初始化 OpenGL 上下文、着色器程序、3D 模型等组件。
    ///
    /// # 返回
    ///
    /// 返回一个配置好的渲染器实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use skinviewer::Renderer;
    ///
    /// let renderer = Renderer::new();
    /// ```
    pub fn new() -> Self {
        let context = ContextBuilder::new()
            .build_osmesa(glutin::dpi::PhysicalSize::new(800, 600))
            .unwrap();
        let context = unsafe { context.make_current().unwrap() };
        let display = Headless::new(context).unwrap();
        let program = Program::from_source(&display, VERTEX_SHADER, FRAGMENT_SHADER, None).unwrap();

        let params = DrawParameters {
            depth: glium::Depth {
                test: DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: BackfaceCullingMode::CullingDisabled, // 禁用背面剔除
            blend: glium::Blend::alpha_blending(),                  // 启用alpha混合
            ..Default::default()
        };

        let model = Model::load_from_obj(&display, "resources/player.obj").unwrap();

        Self {
            display,
            program,
            params,
            skin: None,
            skin_type: SkinType::Default,
            model,
            cape: None,
            nametag: None,
            posture: DefaultPostures::STAND,
            yaw: 0.0,
            pitch: 0.0,
            scale: 1.0,
        }
    }

    /// 设置皮肤纹理
    ///
    /// # 参数
    ///
    /// * `skin` - 要设置的皮肤纹理
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    pub fn set_skin(&mut self, skin: Texture) -> &mut Self {
        self.skin = Some(skin);
        self
    }

    /// 从文件加载并设置皮肤
    ///
    /// # 参数
    ///
    /// * `path` - 皮肤文件路径
    ///
    /// # 返回
    ///
    /// 返回 `Result<&mut Self, Box<dyn std::error::Error>>` 以支持链式调用
    ///
    /// # 示例
    ///
    /// ```rust
    /// let mut renderer = Renderer::new();
    /// renderer.load_skin_from_file("path/to/skin.png")?;
    /// ```
    pub fn load_skin_from_file(
        &mut self,
        path: &str,
    ) -> Result<&mut Self, Box<dyn std::error::Error>> {
        let skin = Texture::load_from_file(&self.display, path)?;
        self.skin = Some(skin);
        Ok(self)
    }

    /// 设置皮肤类型
    ///
    /// # 参数
    ///
    /// * `skin_type` - 皮肤类型（Default 或 Slim）
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    pub fn set_skin_type(&mut self, skin_type: SkinType) -> &mut Self {
        self.skin_type = skin_type;
        self
    }

    /// 设置名称标签
    ///
    /// # 参数
    ///
    /// * `nametag` - 名称标签文本
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    pub fn set_nametag(&mut self, nametag: impl Into<String>) -> &mut Self {
        self.nametag = Some(nametag.into());
        self
    }

    /// 设置角色姿势
    ///
    /// # 参数
    ///
    /// * `posture` - 要设置的姿势
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use skinviewer::{Renderer, DefaultPostures};
    ///
    /// let mut renderer = Renderer::new();
    /// renderer.set_posture(DefaultPostures::STAND);
    /// ```
    pub fn set_posture(&mut self, posture: Posture) -> &mut Self {
        self.posture = posture;
        self
    }

    /// 设置摄像机水平旋转角度
    ///
    /// # 参数
    ///
    /// * `yaw` - 水平旋转角度（0~360）
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    pub fn set_yaw(&mut self, yaw: f32) -> &mut Self {
        self.yaw = yaw;
        self
    }

    /// 设置摄像机垂直旋转角度
    ///
    /// # 参数
    ///
    /// * `pitch` - 垂直旋转角度（0~180）
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    pub fn set_pitch(&mut self, pitch: f32) -> &mut Self {
        self.pitch = pitch;
        self
    }

    /// 设置缩放比例
    ///
    /// # 参数
    ///
    /// * `scale` - 缩放比例（0~1）
    ///
    /// # 返回
    ///
    /// 返回 `&mut Self` 以支持链式调用
    pub fn set_scale(&mut self, scale: f32) -> &mut Self {
        self.scale = scale;
        self
    }

    /// 渲染到图像缓冲区
    ///
    /// # 参数
    ///
    /// * `width` - 输出图像宽度
    /// * `height` - 输出图像高度
    /// * `scale` - 模型缩放比例
    ///
    /// # 返回
    ///
    /// 返回渲染的图像缓冲区
    ///
    /// # 错误
    ///
    /// 如果渲染过程中出现错误，返回相应的错误信息
    pub fn render(
        &self,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
        // 创建离屏渲染纹理和深度缓冲
        let render_texture = Texture2d::empty(&self.display, width, height)?;
        let depth_buffer = DepthRenderBuffer::new(
            &self.display,
            glium::texture::DepthFormat::I24,
            width,
            height,
        )?;
        let mut framebuffer =
            SimpleFrameBuffer::with_depth_buffer(&self.display, &render_texture, &depth_buffer)?;
        framebuffer.clear_color_and_depth((0.2, 0.2, 0.4, 1.0), 1.0);

        let aspect_ratio = height as f32 / width as f32;
        let fov: f32 = std::f32::consts::PI / 3.0;
        let zfar = 1024.0;
        let znear = 0.1;
        let f = 1.0 / (fov / 2.0).tan();
        let perspective = [
            [f * aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
            [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
        ];

        let y_flip = std::f32::consts::PI; // 180度
        let model_matrix = [
            [y_flip.cos() * scale, 0.0, y_flip.sin() * scale, 0.0],
            [0.0, scale, 0.0, 0.0],
            [-y_flip.sin() * scale, 0.0, y_flip.cos() * scale, 0.0],
            [0.0, -0.8, 0.0, 1.0f32],
        ];

        let skin_texture = self.skin.as_ref().ok_or("No skin texture available")?;
        let uniforms = uniform! {
            perspective: perspective,
            view: view_matrix(&[0.0, 1.0, 4.0], &[0.0, -0.2, -1.0], &[0.0, 1.0, 0.0]),
            model: model_matrix,
            texture1: skin_texture.texture.sampled()
                .wrap_function(Repeat)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
        };

        framebuffer.draw(
            &self.model.vertices,
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &self.params,
        )?;

        // 读取像素数据
        let raw: Vec<Vec<(u8, u8, u8, u8)>> = render_texture.read();
        let mut img_buf = ImageBuffer::new(width, height);
        for (y, row) in raw.iter().enumerate() {
            let flipped_y = height as usize - 1 - y;
            for (x, pixel) in row.iter().enumerate() {
                img_buf.put_pixel(
                    x as u32,
                    flipped_y as u32,
                    Rgba([pixel.0, pixel.1, pixel.2, pixel.3]),
                );
            }
        }
        Ok(img_buf)
    }

    /// 渲染并保存到图片文件
    ///
    /// # 参数
    ///
    /// * `filename` - 输出文件名
    /// * `size` - 输出图片尺寸 (宽度, 高度)
    ///
    /// # 返回
    ///
    /// 成功时返回 `Ok(())`，失败时返回错误信息
    ///
    /// # 示例
    ///
    /// ```rust
    /// use skinviewer::Renderer;
    ///
    /// let renderer = Renderer::new();
    /// renderer.render_to_image("output.png", (800, 600));
    /// ```
    pub fn render_to_image(
        &self,
        filename: &str,
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image_buffer = self.render(size.0, size.1, self.scale)?;
        image_buffer.save_with_format(filename, ImageFormat::Png)?;
        Ok(())
    }
}
