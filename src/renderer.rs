use glium::draw_parameters::{BackfaceCullingMode, DepthTest};
use glium::glutin::surface::WindowSurface;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::uniforms::SamplerWrapFunction::Repeat;
use glium::{uniform, Display, DrawParameters, Program, Surface};
use std::time::Instant;

use crate::model::Model;
use crate::texture::Texture;
use crate::utils::view_matrix;

pub struct Renderer {
    program: Program,
    params: DrawParameters<'static>,
    start_time: Instant,
}

impl Renderer {
    pub fn new(
        display: &Display<WindowSurface>,
        vertex_shader: &str,
        fragment_shader: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 编译着色器
        let program = Program::from_source(display, vertex_shader, fragment_shader, None)?;

        // 设置绘制参数
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

        Ok(Renderer {
            program,
            params,
            start_time: Instant::now(),
        })
    }

    pub fn render(
        &self,
        display: &Display<WindowSurface>,
        model: &Model,
        texture: &Texture,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 计算基于时间的旋转，降低旋转速度
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let rotation_angle = elapsed * 0.3; // 降低旋转速度

        // 绘制模型
        let mut target = display.draw();
        // 使用深蓝色背景以更好地展示模型
        target.clear_color_and_depth((0.2, 0.2, 0.4, 1.0), 1.0);

        let perspective = {
            let (width, height) = target.get_dimensions();
            let aspect_ratio = height as f32 / width as f32;

            let fov: f32 = std::f32::consts::PI / 3.0;
            let zfar = 1024.0;
            let znear = 0.1;

            let f = 1.0 / (fov / 2.0).tan();

            [
                [f * aspect_ratio, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
            ]
        };

        // 调整相机位置以更好地查看模型
        // 将相机置于稍高一点的位置，并微微向下看
        let view = view_matrix(&[0.0, 0.3, 3.0], &[0.0, -0.2, -1.0], &[0.0, 1.0, 0.0]);

        // 创建带旋转的模型矩阵
        let scale = 1.2; // 缩放比例
        let model_matrix = [
            [
                rotation_angle.cos() * scale,
                0.0,
                rotation_angle.sin() * scale,
                0.0,
            ],
            [0.0, scale, 0.0, 0.0],
            [
                -rotation_angle.sin() * scale,
                0.0,
                rotation_angle.cos() * scale,
                0.0,
            ],
            [0.0, -0.8, 0.0, 1.0f32], // 将模型向下移动更多
        ];

        let uniforms = uniform! {
            perspective: perspective,
            view: view,
            model: model_matrix,
            texture1: texture.texture.sampled()
                .wrap_function(Repeat)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
        };

        target.draw(
            &model.vertices,
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &self.params,
        )?;

        target.finish()?;

        Ok(())
    }
}
