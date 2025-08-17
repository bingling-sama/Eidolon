use crate::camera::Camera;
use crate::character::Character;
use crate::constants::{FRAGMENT_SHADER, VERTEX_SHADER};
use crate::model::{BodyPart, Model}; // Updated import
use cgmath::{Matrix4, Rad, Vector3}; // Import cgmath
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
    model: Model, // Updated to use the new Model struct
}

impl Renderer {
    /// 创建新的渲染器实例
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
            backface_culling: BackfaceCullingMode::CullingDisabled,
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        // Load the new model structure
        let model = Model::load_from_obj(&display, "resources/slim.obj").unwrap();

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

    /// Helper function to draw a single body part with a given transform
    fn draw_body_part(
        &self,
        framebuffer: &mut SimpleFrameBuffer,
        part: &BodyPart,
        transform: &Matrix4<f32>,
        view: &Matrix4<f32>,
        perspective: &Matrix4<f32>,
        skin_texture: &Texture2d,
    ) -> Result<(), glium::DrawError> {
        let perspective_arr: [[f32; 4]; 4] = (*perspective).into();
        let view_arr: [[f32; 4]; 4] = (*view).into();
        let model_arr: [[f32; 4]; 4] = (*transform).into();

        let uniforms = uniform! {
            perspective: perspective_arr,
            view: view_arr,
            model: model_arr,
            texture1: skin_texture.sampled()
                .wrap_function(Repeat)
                .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
        };

        // Draw the main part
        framebuffer.draw(
            &part.main.vertices,
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &self.params,
        )?;

        // Draw the layer part
        framebuffer.draw(
            &part.layer.vertices,
            &NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniforms,
            &self.params,
        )?;

        Ok(())
    }

    pub fn render(
        &self,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
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

        let perspective: [[f32; 4]; 4] = camera.get_projection_matrix(width, height).into();
        let view: [[f32; 4]; 4] = camera.get_view_matrix().into();
        let perspective_matrix: Matrix4<f32> = perspective.into();
        let view_matrix: Matrix4<f32> = view.into();

        let skin_texture = character.skin.as_ref().ok_or("No skin texture available")?;

        // --- Transformation Matrices ---
        let translation = Matrix4::from_translation([0.0, -0.8, 0.0].into());
        let scale = Matrix4::from_scale(camera.scale);
        let base_model_matrix = translation * scale;

        // --- Draw each body part using posture data with pivot points ---
        let posture = &character.posture;

        // Body (no rotation, base of all transforms)
        let body_transform = base_model_matrix;
        self.draw_body_part(
            &mut framebuffer,
            &self.model.body,
            &body_transform,
            &view_matrix,
            &perspective_matrix,
            &skin_texture.texture,
        )?;

        // Head
        let head_pivot = Vector3::new(0.0, 1.5, 0.0);
        let head_yaw_rad = (posture.head_yaw - 90.0).to_radians();
        let head_pitch_rad = (posture.head_pitch - 90.0).to_radians();
        let head_rotation =
            Matrix4::from_angle_y(Rad(head_yaw_rad)) * Matrix4::from_angle_x(Rad(head_pitch_rad));
        let head_transform = base_model_matrix
            * Matrix4::from_translation(head_pivot)
            * head_rotation
            * Matrix4::from_translation(-head_pivot);
        self.draw_body_part(
            &mut framebuffer,
            &self.model.head,
            &head_transform,
            &view_matrix,
            &perspective_matrix,
            &skin_texture.texture,
        )?;

        // Right Arm
        let right_arm_pivot = Vector3::new(0.3125, 1.375, 0.0);
        let right_arm_roll_rad = posture.right_arm_roll.to_radians();
        let right_arm_pitch_rad = posture.right_arm_pitch.to_radians();
        let right_arm_rotation = Matrix4::from_angle_z(Rad(right_arm_roll_rad))
            * Matrix4::from_angle_x(Rad(right_arm_pitch_rad));
        let right_arm_transform = base_model_matrix
            * Matrix4::from_translation(right_arm_pivot)
            * right_arm_rotation
            * Matrix4::from_translation(-right_arm_pivot);
        self.draw_body_part(
            &mut framebuffer,
            &self.model.right_arm,
            &right_arm_transform,
            &view_matrix,
            &perspective_matrix,
            &skin_texture.texture,
        )?;

        // Left Arm
        let left_arm_pivot = Vector3::new(-0.3125, 1.375, 0.0);
        let left_arm_roll_rad = -posture.left_arm_roll.to_radians();
        let left_arm_pitch_rad = posture.left_arm_pitch.to_radians();
        let left_arm_rotation = Matrix4::from_angle_z(Rad(left_arm_roll_rad))
            * Matrix4::from_angle_x(Rad(left_arm_pitch_rad));
        let left_arm_transform = base_model_matrix
            * Matrix4::from_translation(left_arm_pivot)
            * left_arm_rotation
            * Matrix4::from_translation(-left_arm_pivot);
        self.draw_body_part(
            &mut framebuffer,
            &self.model.left_arm,
            &left_arm_transform,
            &view_matrix,
            &perspective_matrix,
            &skin_texture.texture,
        )?;

        // Right Leg
        let right_leg_pivot = Vector3::new(0.125, 0.75, 0.0);
        let right_leg_pitch_rad = (posture.right_leg_pitch - 90.0).to_radians();
        let right_leg_rotation = Matrix4::from_angle_x(Rad(right_leg_pitch_rad));
        let right_leg_transform = base_model_matrix
            * Matrix4::from_translation(right_leg_pivot)
            * right_leg_rotation
            * Matrix4::from_translation(-right_leg_pivot);
        self.draw_body_part(
            &mut framebuffer,
            &self.model.right_leg,
            &right_leg_transform,
            &view_matrix,
            &perspective_matrix,
            &skin_texture.texture,
        )?;

        // Left Leg
        let left_leg_pivot = Vector3::new(-0.125, 0.75, 0.0);
        let left_leg_pitch_rad = (posture.left_leg_pitch - 90.0).to_radians();
        let left_leg_rotation = Matrix4::from_angle_x(Rad(left_leg_pitch_rad));
        let left_leg_transform = base_model_matrix
            * Matrix4::from_translation(left_leg_pivot)
            * left_leg_rotation
            * Matrix4::from_translation(-left_leg_pivot);
        self.draw_body_part(
            &mut framebuffer,
            &self.model.left_leg,
            &left_leg_transform,
            &view_matrix,
            &perspective_matrix,
            &skin_texture.texture,
        )?;

        // Read pixels from framebuffer
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
