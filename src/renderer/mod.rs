//! WGPU renderer: headless RGBA readback and windowed surface preview, shared skin pipeline.

mod output;
mod pipeline;
mod readback;
mod uniforms;

pub use output::OutputFormat;

use std::cell::RefCell;
use std::sync::Arc;

use image::{ImageBuffer, Rgba};
use winit::window::Window;

use crate::camera::Camera;
use crate::character::{Character, SkinType};
use crate::model::Model;
use crate::texture::Texture;

use pipeline::{create_pipeline, DEPTH_FORMAT, RENDER_TARGET_FORMAT};
use uniforms::compute_body_part_uniforms;

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
    /// Cached depth buffer; recreated when dimensions change (avoids per-frame alloc in windowed preview).
    cached_depth_texture: RefCell<Option<(wgpu::Texture, u32, u32)>>,
}

impl Renderer {
    /// Headless renderer (no surface): offscreen `Rgba8Unorm` target and CPU readback.
    #[allow(clippy::new_without_default)]
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

    /// Windowed renderer: creates a surface and optional second pipeline if the swapchain format differs.
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
                            std::mem::size_of::<uniforms::Uniforms>() as u64,
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

        let alignment = device.limits().min_uniform_buffer_offset_alignment;
        let uniform_size = std::mem::size_of::<uniforms::Uniforms>() as u32;
        let aligned_size = uniform_size.div_ceil(alignment) * alignment;
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
            cached_depth_texture: RefCell::new(None),
        }
    }

    /// Load a skin PNG and build GPU resources (same path as CLI `--texture`).
    pub fn load_texture(&self, path: &str) -> Result<Texture, Box<dyn std::error::Error>> {
        Texture::load_from_file(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            path,
        )
    }

    #[allow(clippy::too_many_arguments)]
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

        let uniforms = compute_body_part_uniforms(character, camera, width, height);

        for (i, uniform) in uniforms.iter().enumerate() {
            let offset = (i as u64) * (self.uniform_aligned_size as u64);
            self.queue
                .write_buffer(&self.uniform_buffer, offset, bytemuck::bytes_of(uniform));
        }

        let depth_view = {
            let mut cache = self.cached_depth_texture.borrow_mut();
            let need_new = match cache.as_ref() {
                None => true,
                Some((_, w, h)) => *w != width || *h != height,
            };
            if need_new {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
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
                *cache = Some((texture, width, height));
            }
            cache
                .as_ref()
                .unwrap()
                .0
                .create_view(&wgpu::TextureViewDescriptor::default())
        };

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

    /// Render to an offscreen texture and return an RGBA [`image::ImageBuffer`] (blocking map readback).
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

        let (output_buffer, padded_bytes_per_row) =
            readback::create_output_buffer(&self.device, width, height);

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

        readback::copy_render_target_to_buffer(
            &mut encoder,
            &render_texture,
            &output_buffer,
            width,
            height,
            padded_bytes_per_row,
        );

        self.queue.submit(Some(encoder.finish()));

        readback::map_output_buffer_to_rgba(
            &self.device,
            &output_buffer,
            width,
            height,
            padded_bytes_per_row,
        )
    }

    /// Present one frame to the window surface (expects `new_windowed`).
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

    /// Update surface extent after a resize; no-op if not windowed or size is zero.
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

    /// Calls [`Renderer::render`], then saves using [`OutputFormat`].
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
