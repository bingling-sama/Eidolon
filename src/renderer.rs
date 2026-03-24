use std::sync::Arc;

use cgmath::{Matrix4, Rad, Vector3};
use image::{ImageBuffer, ImageFormat, Rgba};
use winit::window::Window;

use crate::camera::Camera;
use crate::character::{Character, SkinType};
use crate::constants::SHADER;
use crate::model::{Model, TexturedVertex};
use crate::texture::Texture;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const RENDER_TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// 输出图片格式
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// PNG 格式
    Png,
    /// WebP 格式
    WebP,
}

impl OutputFormat {
    pub fn as_image_format(&self) -> ImageFormat {
        match self {
            OutputFormat::Png => ImageFormat::Png,
            OutputFormat::WebP => ImageFormat::WebP,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    perspective: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    offset: f32,
    _padding: [f32; 3],
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_aligned_size: u32,
    slim_model: Model,
    default_model: Model,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    surface_pipeline: Option<wgpu::RenderPipeline>,
}

fn create_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Skin Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[TexturedVertex::desc()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

impl Renderer {
    /// 创建新的 Headless 渲染器实例
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Eidolon Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            },
        ))
        .expect("Failed to create device");

        Self::init_with_device(device, queue, None)
    }

    /// 创建新的 Windowed 渲染器实例
    pub fn new_windowed(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Eidolon Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            },
        ))
        .expect("Failed to create device");

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        // Prefer non-sRGB surface format to match headless rendering behavior
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self::init_with_device(device, queue, Some((surface, config, surface_format)))
    }

    fn init_with_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_info: Option<(
            wgpu::Surface<'static>,
            wgpu::SurfaceConfiguration,
            wgpu::TextureFormat,
        )>,
    ) -> Self {
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniforms>() as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Skin Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = create_pipeline(&device, &pipeline_layout, RENDER_TARGET_FORMAT);

        let surface_pipeline = surface_info.as_ref().and_then(|(_, _, format)| {
            if *format != RENDER_TARGET_FORMAT {
                Some(create_pipeline(&device, &pipeline_layout, *format))
            } else {
                None
            }
        });

        // Dynamic uniform buffer for 6 body parts
        let alignment = device.limits().min_uniform_buffer_offset_alignment;
        let uniform_size = std::mem::size_of::<Uniforms>() as u32;
        let aligned_size = ((uniform_size + alignment - 1) / alignment) * alignment;
        let num_body_parts = 6u32;

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Uniform Buffer"),
            size: (num_body_parts * aligned_size) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(uniform_size as u64),
                }),
            }],
        });

        let slim_model = Model::load_from_obj(&device, "resources/slim.obj").unwrap();
        let default_model = Model::load_from_obj(&device, "resources/classic.obj").unwrap();

        let (surface, surface_config) = match surface_info {
            Some((s, c, _)) => (Some(s), Some(c)),
            None => (None, None),
        };

        Self {
            device,
            queue,
            pipeline,
            texture_bind_group_layout,
            sampler,
            uniform_buffer,
            uniform_bind_group,
            uniform_aligned_size: aligned_size,
            slim_model,
            default_model,
            surface,
            surface_config,
            surface_pipeline,
        }
    }

    /// 加载皮肤纹理
    pub fn load_texture(&self, path: &str) -> Result<Texture, Box<dyn std::error::Error>> {
        Texture::load_from_file(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            path,
        )
    }

    fn compute_body_part_uniforms(
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> [Uniforms; 6] {
        let perspective: [[f32; 4]; 4] = camera.get_projection_matrix(width, height);
        let view: [[f32; 4]; 4] = camera.get_view_matrix();

        let translation = Matrix4::from_translation(character.position);
        let rotation_matrix = Matrix4::from_angle_x(Rad(character.rotation.x.to_radians()))
            * Matrix4::from_angle_y(Rad(character.rotation.y.to_radians()))
            * Matrix4::from_angle_z(Rad(character.rotation.z.to_radians()));
        let scale = Matrix4::from_scale(camera.scale);
        let base_model_matrix = translation * rotation_matrix * scale;

        let posture = &character.posture;

        let make_uniforms = |model_matrix: Matrix4<f32>, offset: f32| -> Uniforms {
            Uniforms {
                perspective,
                view,
                model: model_matrix.into(),
                offset,
                _padding: [0.0; 3],
            }
        };

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

        // Right Leg
        let right_leg_pivot = Vector3::new(0.125, 0.75, 0.0);
        let right_leg_pitch_rad = (posture.right_leg_pitch - 90.0).to_radians();
        let right_leg_rotation = Matrix4::from_angle_x(Rad(right_leg_pitch_rad));
        let right_leg_transform = base_model_matrix
            * Matrix4::from_translation(right_leg_pivot)
            * right_leg_rotation
            * Matrix4::from_translation(-right_leg_pivot);

        // Left Leg
        let left_leg_pivot = Vector3::new(-0.125, 0.75, 0.0);
        let left_leg_pitch_rad = (posture.left_leg_pitch - 90.0).to_radians();
        let left_leg_rotation = Matrix4::from_angle_x(Rad(left_leg_pitch_rad));
        let left_leg_transform = base_model_matrix
            * Matrix4::from_translation(left_leg_pivot)
            * left_leg_rotation
            * Matrix4::from_translation(-left_leg_pivot);

        // Body (no additional rotation)
        let body_transform = base_model_matrix;

        [
            make_uniforms(head_transform, 0.0),
            make_uniforms(right_arm_transform, 0.0),
            make_uniforms(left_arm_transform, 0.0),
            make_uniforms(right_leg_transform, 0.0),
            make_uniforms(left_leg_transform, 0.0),
            make_uniforms(body_transform, 0.0001),
        ]
    }

    fn encode_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) {
        let skin_texture = character.skin.as_ref().expect("No skin texture available");

        let model = match character.skin_type {
            SkinType::Slim => &self.slim_model,
            SkinType::Classic => &self.default_model,
        };

        let uniforms = Self::compute_body_part_uniforms(character, camera, width, height);

        for (i, uniform) in uniforms.iter().enumerate() {
            let offset = (i as u64) * (self.uniform_aligned_size as u64);
            self.queue
                .write_buffer(&self.uniform_buffer, offset, bytemuck::bytes_of(uniform));
        }

        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Draw order matches original: head, right_arm, left_arm, right_leg, left_leg, body
        let body_parts = [
            &model.head,
            &model.right_arm,
            &model.left_arm,
            &model.right_leg,
            &model.left_leg,
            &model.body,
        ];

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.2,
                            b: 0.4,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(1, &skin_texture.bind_group, &[]);

            for (i, body_part) in body_parts.iter().enumerate() {
                let dynamic_offset = (i as u32) * self.uniform_aligned_size;
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[dynamic_offset]);

                render_pass.set_vertex_buffer(0, body_part.main.vertex_buffer.slice(..));
                render_pass.draw(0..body_part.main.vertex_count, 0..1);

                render_pass.set_vertex_buffer(0, body_part.layer.vertex_buffer.slice(..));
                render_pass.draw(0..body_part.layer.vertex_count, 0..1);
            }
        }
    }

    /// 渲染角色到图像缓冲区（Headless）
    pub fn render(
        &self,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
        let render_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: RENDER_TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.encode_render_pass(
            &mut encoder,
            &texture_view,
            &self.pipeline,
            character,
            camera,
            width,
            height,
        );

        // Copy render texture to a mappable buffer for pixel readback
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = bytes_per_pixel * width;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        // Map the output buffer and read pixels
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::PollType::Wait).ok();
        rx.recv().unwrap()?;

        let data = buffer_slice.get_mapped_range();
        let mut img_buf = ImageBuffer::new(width, height);

        for y in 0..height {
            let row_start = (y * padded_bytes_per_row) as usize;
            for x in 0..width {
                let offset = row_start + (x * bytes_per_pixel) as usize;
                img_buf.put_pixel(
                    x,
                    y,
                    Rgba([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]),
                );
            }
        }

        drop(data);
        output_buffer.unmap();

        Ok(img_buf)
    }

    /// 渲染角色到窗口 Surface（Windowed）
    pub fn render_frame(
        &self,
        character: &Character,
        camera: &Camera,
    ) -> Result<(), wgpu::SurfaceError> {
        let surface = self.surface.as_ref().expect("Not in windowed mode");
        let config = self.surface_config.as_ref().unwrap();

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline = self.surface_pipeline.as_ref().unwrap_or(&self.pipeline);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Frame Encoder"),
            });

        self.encode_render_pass(
            &mut encoder,
            &view,
            pipeline,
            character,
            camera,
            config.width,
            config.height,
        );

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    /// 调整窗口大小时重新配置 Surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let (Some(surface), Some(config)) = (&self.surface, &mut self.surface_config) {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
    }

    /// 渲染角色并保存为图片文件
    pub fn render_to_image(
        &self,
        character: &Character,
        camera: &Camera,
        filename: &str,
        size: (u32, u32),
        format: OutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image_buffer = self.render(character, camera, size.0, size.1)?;
        image_buffer.save_with_format(filename, format.as_image_format())?;
        Ok(())
    }
}
