use crate::camera::Camera;
use crate::character::Character;
use crate::constants::{FRAGMENT_SHADER, VERTEX_SHADER};
use crate::model::Model;
use glium::backend::glutin::headless::Headless;
use glium::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::uniforms::SamplerWrapFunction::Repeat;
use glium::{uniform, BackfaceCullingMode, DepthTest, DrawParameters, Program, Surface, Texture2d};
use glutin::platform::unix::HeadlessContextExt;
use glutin::ContextBuilder;
use image::{ImageBuffer, ImageFormat, Rgba};

/// 输出图片格式
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// PNG 格式
    Png,
    /// WebP 格式
    WebP,
}

pub struct Renderer {
    /// OpenGL 显示上下文
    display: Headless,
    /// 着色器程序
    program: Program,
    /// 绘制参数
    params: DrawParameters<'static>,
    /// 3D 模型
    model: Model,
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
            model,
        }
    }

    pub fn get_display(&self) -> &Headless {
        &self.display
    }

    pub fn render(
        &self,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
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

        let perspective = camera.get_projection_matrix(width, height);
        let view = camera.get_view_matrix();

        let y_flip = std::f32::consts::PI; // 180度
        let model_matrix = [
            [
                y_flip.cos() * camera.scale,
                0.0,
                y_flip.sin() * camera.scale,
                0.0,
            ],
            [0.0, camera.scale, 0.0, 0.0],
            [
                -y_flip.sin() * camera.scale,
                0.0,
                y_flip.cos() * camera.scale,
                0.0,
            ],
            [0.0, -0.8, 0.0, 1.0f32],
        ];

        let skin_texture = character.skin.as_ref().ok_or("No skin texture available")?;
        let uniforms = uniform! {
            perspective: perspective,
            view: view,
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

    pub fn render_to_image(
        &self,
        character: &Character,
        camera: &Camera,
        filename: &str,
        size: (u32, u32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image_buffer = self.render(character, camera, size.0, size.1)?;
        image_buffer.save_with_format(filename, ImageFormat::Png)?;
        Ok(())
    }
}
