//! WGPU renderer: headless RGBA readback and windowed surface preview, shared skin pipeline.

pub(crate) mod nametag;
mod pipeline;
mod readback;
mod uniforms;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use image::{ImageBuffer, Rgba};
use wgpu::util::DeviceExt;
#[cfg(all(feature = "winit", not(target_arch = "wasm32")))]
use winit::window::Window;

use crate::back_equipment::{BackEquipmentAtlas, BackEquipmentKind};
use crate::body::Body;
use crate::camera::Camera;
use crate::character::{Character, SkinType};
use crate::error::EidolonError;
use crate::model::ModelCatalog;
use crate::texture::Texture;

use nametag::{compute_nametag_uniform, nametag_passes, CachedNametag, NametagPass};
use pipeline::{create_nametag_pipeline, create_pipeline, DEPTH_FORMAT, RENDER_TARGET_FORMAT};
use uniforms::{
    body_part_ref, compute_body_part_uniforms, compute_cape_uniform, compute_elytra_uniforms,
    CAPE_UNIFORM_INDEX, LEFT_ELYTRA_UNIFORM_INDEX, NAMETAG_UNIFORM_INDEX, PART_CONFIGS,
    RIGHT_ELYTRA_UNIFORM_INDEX, UNIFORM_COUNT,
};

static NEXT_RENDERER_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct Renderer {
    context_id: u64,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    nametag_pipeline: wgpu::RenderPipeline,
    nametag_see_through_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_aligned_size: u32,
    models: ModelCatalog,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    surface_pipeline: Option<wgpu::RenderPipeline>,
    nametag_surface_pipeline: Option<wgpu::RenderPipeline>,
    nametag_see_through_surface_pipeline: Option<wgpu::RenderPipeline>,
    /// Cached nametag textures and meshes (indexed by text).
    cached_nametags: Mutex<std::collections::HashMap<String, Arc<CachedNametag>>>,
    /// RGBA clear color for the render pass background. Default: transparent black.
    clear_color: [f64; 4],
    /// Cached offscreen render targets (indexed by (width, height)).
    offscreen_pool: Mutex<std::collections::HashMap<(u32, u32), OffscreenTarget>>,
    /// Cached windowed depth texture & view.
    windowed_depth: Mutex<Option<(u32, u32, wgpu::Texture, wgpu::TextureView)>>,
}

pub(crate) struct OffscreenTarget {
    pub width: u32,
    pub height: u32,
    pub render_texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub staging_buffer: wgpu::Buffer,
    pub padded_bytes_per_row: u32,
}

impl OffscreenTarget {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self, EidolonError> {
        let layout = readback::output_layout(width, height)?;
        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
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

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
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

        let staging_buffer = readback::create_output_buffer(device, &layout);

        Ok(Self {
            width,
            height,
            render_texture,
            texture_view,
            depth_texture,
            depth_view,
            staging_buffer,
            padded_bytes_per_row: layout.padded_bytes_per_row,
        })
    }
}

impl Renderer {
    /// Shared wgpu bootstrap: adapter + device + queue from an existing Instance.
    #[cfg(not(target_arch = "wasm32"))]
    fn create_wgpu_device(
        instance: &wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface>,
    ) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter), EidolonError> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface,
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .map_err(|e| EidolonError::gpu(format!("failed to request adapter: {e}")))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Eidolon Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
            ..Default::default()
        }))
        .map_err(|e| EidolonError::gpu(format!("failed to request GPU device: {e}")))?;
        Ok((device, queue, adapter))
    }

    /// Headless renderer (no surface): offscreen `Rgba8Unorm` target and CPU readback.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Self, EidolonError> {
        let instance = wgpu::Instance::default();
        let (device, queue, _) = Self::create_wgpu_device(&instance, None)?;
        Self::init_with_device(device, queue, None)
    }

    /// Windowed renderer: creates a surface and optional second pipeline if the swapchain format differs.
    #[cfg(all(feature = "winit", not(target_arch = "wasm32")))]
    pub fn new_windowed(window: Arc<Window>) -> Result<Self, EidolonError> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| EidolonError::gpu(format!("failed to create surface: {e}")))?;
        let (device, queue, adapter) = Self::create_wgpu_device(&instance, Some(&surface))?;

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
            alpha_mode: surface_caps
                .alpha_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::CompositeAlphaMode::PreMultiplied)
                .or_else(|| {
                    surface_caps
                        .alpha_modes
                        .iter()
                        .copied()
                        .find(|m| *m == wgpu::CompositeAlphaMode::PostMultiplied)
                })
                .unwrap_or(surface_caps.alpha_modes[0]),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        Self::init_with_device(device, queue, Some((surface, config, surface_format)))
    }

    /// Headless renderer for WASM: offscreen target with async adapter/device creation.
    #[cfg(target_arch = "wasm32")]
    pub async fn new_async() -> Result<Self, EidolonError> {
        let instance = wgpu::util::new_instance_with_webgpu_detection(
            wgpu::InstanceDescriptor::new_without_display_handle(),
        )
        .await;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|e| EidolonError::gpu(format!("failed to request adapter: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Eidolon Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                ..Default::default()
            })
            .await
            .map_err(|e| EidolonError::gpu(format!("failed to request GPU device: {e}")))?;
        Self::init_with_device(device, queue, None)
    }

    /// Windowed renderer for WASM: creates a surface from a canvas and configures it.
    ///
    /// `canvas` can be any type that converts into a wgpu surface target
    /// (e.g. `web_sys::HtmlCanvasElement` or `wgpu::SurfaceTarget`).
    #[cfg(target_arch = "wasm32")]
    pub async fn new_windowed_async(
        canvas: impl Into<wgpu::SurfaceTarget<'static>>,
    ) -> Result<Self, EidolonError> {
        let instance = wgpu::util::new_instance_with_webgpu_detection(
            wgpu::InstanceDescriptor::new_without_display_handle(),
        )
        .await;
        let surface = instance
            .create_surface(canvas)
            .map_err(|e| EidolonError::gpu(format!("failed to create surface: {e}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|e| EidolonError::gpu(format!("failed to request adapter: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Eidolon Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                ..Default::default()
            })
            .await
            .map_err(|e| EidolonError::gpu(format!("failed to request GPU device: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 512,
            height: 512,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps
                .alpha_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::CompositeAlphaMode::PreMultiplied)
                .or_else(|| {
                    caps.alpha_modes
                        .iter()
                        .copied()
                        .find(|m| *m == wgpu::CompositeAlphaMode::PostMultiplied)
                })
                .unwrap_or(caps.alpha_modes[0]),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
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
    ) -> Result<Self, EidolonError> {
        // Log wgpu errors (shader compilation, pipeline creation, etc.)
        // instead of letting them silently drop on the GPU error queue.
        device.on_uncaptured_error(Arc::new(|error| {
            log::error!("wgpu error: {:?}", error);
        }));

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<
                            uniforms::Uniforms,
                        >() as u64),
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
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[
                Some(&uniform_bind_group_layout),
                Some(&texture_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let pipeline = create_pipeline(&device, &pipeline_layout, RENDER_TARGET_FORMAT);
        let nametag_pipeline = create_nametag_pipeline(
            &device,
            &pipeline_layout,
            RENDER_TARGET_FORMAT,
            NametagPass::Normal,
        );
        let nametag_see_through_pipeline = create_nametag_pipeline(
            &device,
            &pipeline_layout,
            RENDER_TARGET_FORMAT,
            NametagPass::SeeThrough,
        );

        let surface_pipeline = surface_info.as_ref().and_then(|(_, _, format)| {
            if *format != RENDER_TARGET_FORMAT {
                Some(create_pipeline(&device, &pipeline_layout, *format))
            } else {
                None
            }
        });
        let nametag_surface_pipeline = surface_info.as_ref().and_then(|(_, _, format)| {
            if *format != RENDER_TARGET_FORMAT {
                Some(create_nametag_pipeline(
                    &device,
                    &pipeline_layout,
                    *format,
                    NametagPass::Normal,
                ))
            } else {
                None
            }
        });
        let nametag_see_through_surface_pipeline =
            surface_info.as_ref().and_then(|(_, _, format)| {
                if *format != RENDER_TARGET_FORMAT {
                    Some(create_nametag_pipeline(
                        &device,
                        &pipeline_layout,
                        *format,
                        NametagPass::SeeThrough,
                    ))
                } else {
                    None
                }
            });

        let alignment = device.limits().min_uniform_buffer_offset_alignment;
        let uniform_size = std::mem::size_of::<uniforms::Uniforms>() as u32;
        let aligned_size = uniform_size.div_ceil(alignment) * alignment;

        let models = ModelCatalog::new(&device)?;

        let (surface, surface_config) = match surface_info {
            Some((s, c, _)) => (Some(s), Some(c)),
            None => (None, None),
        };

        Ok(Self {
            context_id: NEXT_RENDERER_CONTEXT_ID.fetch_add(1, Ordering::Relaxed),
            device,
            queue,
            pipeline,
            nametag_pipeline,
            nametag_see_through_pipeline,
            texture_bind_group_layout,
            uniform_bind_group_layout,
            sampler,
            uniform_aligned_size: aligned_size,
            models,
            surface,
            surface_config,
            surface_pipeline,
            nametag_surface_pipeline,
            nametag_see_through_surface_pipeline,
            cached_nametags: Mutex::new(std::collections::HashMap::new()),
            clear_color: [0.0, 0.0, 0.0, 0.0],
            offscreen_pool: Mutex::new(std::collections::HashMap::new()),
            windowed_depth: Mutex::new(None),
        })
    }

    fn acquire_offscreen_target(
        &self,
        width: u32,
        height: u32,
    ) -> Result<OffscreenTarget, EidolonError> {
        readback::validate_dimensions(width, height)?;
        if let Ok(mut pool) = self.offscreen_pool.lock() {
            if let Some(target) = pool.remove(&(width, height)) {
                return Ok(target);
            }
        }
        OffscreenTarget::new(&self.device, width, height)
    }

    fn release_offscreen_target(&self, target: OffscreenTarget) {
        if let Ok(mut pool) = self.offscreen_pool.lock() {
            if pool.len() < 4 {
                pool.insert((target.width, target.height), target);
            }
        }
    }

    fn get_or_create_windowed_depth(
        &self,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        if let Ok(mut cache) = self.windowed_depth.lock() {
            if let Some((w, h, texture, view)) = cache.as_ref() {
                if *w == width && *h == height {
                    return (texture.clone(), view.clone());
                }
            }
            let texture = self.create_depth_texture(width, height);
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            *cache = Some((width, height, texture.clone(), view.clone()));
            (texture, view)
        } else {
            let texture = self.create_depth_texture(width, height);
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (texture, view)
        }
    }

    fn create_depth_texture(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
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
        })
    }

    /// Upload a pre-loaded/validated [`RawCapeAtlas`] to GPU texture.
    pub fn upload_cape_atlas(
        &self,
        atlas: &crate::assets::RawCapeAtlas,
    ) -> Result<BackEquipmentAtlas, EidolonError> {
        let texture = Texture::upload_raw(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            atlas.as_raw(),
            atlas.width(),
            atlas.height(),
        )?;
        Ok(BackEquipmentAtlas::new(texture, self.context_id))
    }

    /// Upload a pre-loaded/validated [`RawSkin`] to GPU texture.
    pub fn upload_skin(
        &self,
        skin: &crate::assets::RawSkin,
        skin_type: SkinType,
    ) -> Result<Body, EidolonError> {
        let texture = Texture::upload_raw(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            skin.as_raw(),
            skin.width(),
            skin.height(),
        )?;
        Ok(Body::new(texture, skin_type, self.context_id))
    }

    /// Load and validate the atlas shared by cape and elytra presentation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_back_equipment_atlas(
        &self,
        path: &str,
    ) -> Result<BackEquipmentAtlas, EidolonError> {
        let raw = crate::assets::RawCapeAtlas::from_file(path)?;
        self.upload_cape_atlas(&raw)
    }

    /// Load a back-equipment atlas from PNG bytes.
    pub fn load_back_equipment_atlas_from_memory(
        &self,
        bytes: &[u8],
    ) -> Result<BackEquipmentAtlas, EidolonError> {
        let raw = crate::assets::RawCapeAtlas::from_memory(bytes)?;
        self.upload_cape_atlas(&raw)
    }

    /// Load a body skin and bind its classic or slim geometry selection.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_body(&self, path: &str, skin_type: SkinType) -> Result<Body, EidolonError> {
        let raw = crate::assets::RawSkin::from_file(path)?;
        self.upload_skin(&raw, skin_type)
    }

    /// Load a body skin from PNG bytes.
    pub fn load_body_from_memory(
        &self,
        bytes: &[u8],
        skin_type: SkinType,
    ) -> Result<Body, EidolonError> {
        let raw = crate::assets::RawSkin::from_memory(bytes)?;
        self.upload_skin(&raw, skin_type)
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_render_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        nametag_pipeline: &wgpu::RenderPipeline,
        nametag_see_through_pipeline: &wgpu::RenderPipeline,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) {
        let uniform_size = std::mem::size_of::<uniforms::Uniforms>() as u32;
        let mut uniform_bytes = vec![0u8; (UNIFORM_COUNT * self.uniform_aligned_size) as usize];

        let body = character.body();
        let model = body.map(|body| self.models.body_model(body.skin_type()));

        if body.is_some() {
            let uniforms = compute_body_part_uniforms(character, camera, width, height);
            for (i, uniform) in uniforms.iter().enumerate() {
                let offset = i * (self.uniform_aligned_size as usize);
                let bytes = bytemuck::bytes_of(uniform);
                uniform_bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
            }
        }

        let equipment = character.back_equipment();
        let cape_atlas = equipment
            .filter(|equipment| equipment.kind() == BackEquipmentKind::Cape)
            .map(|equipment| equipment.atlas().texture.as_ref());
        let elytra_atlas = equipment
            .filter(|equipment| equipment.kind() == BackEquipmentKind::Elytra)
            .map(|equipment| equipment.atlas().texture.as_ref());

        if cape_atlas.is_some() {
            let cape_uniform = compute_cape_uniform(character, camera, width, height);
            let offset = (CAPE_UNIFORM_INDEX as usize) * (self.uniform_aligned_size as usize);
            let bytes = bytemuck::bytes_of(&cape_uniform);
            uniform_bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
        }

        if elytra_atlas.is_some() {
            let elytra_uniforms = compute_elytra_uniforms(character, camera, width, height);
            for (index, uniform) in [LEFT_ELYTRA_UNIFORM_INDEX, RIGHT_ELYTRA_UNIFORM_INDEX]
                .into_iter()
                .zip(elytra_uniforms)
            {
                let offset = (index as usize) * (self.uniform_aligned_size as usize);
                let bytes = bytemuck::bytes_of(&uniform);
                uniform_bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
            }
        }

        let nametag_cached = character
            .nametag
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .and_then(|text| {
                let mut cache = self.cached_nametags.lock().unwrap();
                if let Some(cached) = cache.get(text) {
                    Some(Arc::clone(cached))
                } else if let Some(new_cached) = CachedNametag::new(
                    &self.device,
                    &self.queue,
                    &self.texture_bind_group_layout,
                    &self.sampler,
                    text,
                    self.context_id,
                ) {
                    cache.insert(text.to_owned(), Arc::clone(&new_cached));
                    Some(new_cached)
                } else {
                    None
                }
            });

        if nametag_cached.is_some() {
            let uniform = compute_nametag_uniform(character, camera, width, height);
            let offset = (NAMETAG_UNIFORM_INDEX as usize) * (self.uniform_aligned_size as usize);
            let bytes = bytemuck::bytes_of(&uniform);
            uniform_bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
        }

        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame Dynamic Uniform Buffer"),
                contents: &uniform_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(uniform_size as u64),
                }),
            }],
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color[0],
                            g: self.clear_color[1],
                            b: self.clear_color[2],
                            a: self.clear_color[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(pipeline);

            if let (Some(body), Some(model)) = (body, model) {
                render_pass.set_bind_group(1, &body.skin.bind_group, &[]);
                for i in 0..PART_CONFIGS.len() {
                    let body_part = body_part_ref(i, model);
                    let dynamic_offset = (i as u32) * self.uniform_aligned_size;
                    render_pass.set_bind_group(0, &uniform_bind_group, &[dynamic_offset]);

                    render_pass.set_vertex_buffer(0, body_part.main.vertex_buffer.slice(..));
                    render_pass.draw(0..body_part.main.vertex_count, 0..1);

                    render_pass.set_vertex_buffer(0, body_part.layer.vertex_buffer.slice(..));
                    render_pass.draw(0..body_part.layer.vertex_count, 0..1);
                }
            }

            if let Some(cape_atlas) = cape_atlas {
                render_pass.set_bind_group(
                    0,
                    &uniform_bind_group,
                    &[CAPE_UNIFORM_INDEX * self.uniform_aligned_size],
                );
                render_pass.set_bind_group(1, &cape_atlas.bind_group, &[]);
                render_pass
                    .set_vertex_buffer(0, self.models.cape_model().main.vertex_buffer.slice(..));
                render_pass.draw(0..self.models.cape_model().main.vertex_count, 0..1);
            }

            if let Some(elytra_atlas) = elytra_atlas {
                render_pass.set_bind_group(1, &elytra_atlas.bind_group, &[]);
                for (index, wing) in [
                    (
                        LEFT_ELYTRA_UNIFORM_INDEX,
                        &self.models.elytra_model().left_wing,
                    ),
                    (
                        RIGHT_ELYTRA_UNIFORM_INDEX,
                        &self.models.elytra_model().right_wing,
                    ),
                ] {
                    render_pass.set_bind_group(
                        0,
                        &uniform_bind_group,
                        &[index * self.uniform_aligned_size],
                    );
                    render_pass.set_vertex_buffer(0, wing.vertex_buffer.slice(..));
                    render_pass.draw(0..wing.vertex_count, 0..1);
                }
            }

            if let Some(cached) = nametag_cached {
                render_pass.set_bind_group(
                    0,
                    &uniform_bind_group,
                    &[NAMETAG_UNIFORM_INDEX * self.uniform_aligned_size],
                );
                render_pass.set_bind_group(1, &cached.texture.bind_group, &[]);
                render_pass.set_vertex_buffer(0, cached.vertex_buffer.slice(..));
                for pass in nametag_passes(character.nametag_see_through) {
                    render_pass.set_pipeline(match pass {
                        NametagPass::SeeThrough => nametag_see_through_pipeline,
                        NametagPass::Normal => nametag_pipeline,
                    });
                    render_pass.draw(0..cached.vertex_count, 0..1);
                }
            }
        }
    }

    fn validate_character_context(&self, character: &Character) -> Result<(), EidolonError> {
        let body_matches = character
            .body()
            .is_none_or(|body| body.context_id == self.context_id);
        let equipment_matches = character
            .back_equipment()
            .is_none_or(|equipment| equipment.atlas().context_id == self.context_id);
        if body_matches && equipment_matches {
            Ok(())
        } else {
            Err(EidolonError::gpu(
                "character resources belong to a different renderer context",
            ))
        }
    }

    /// Render a character to an offscreen RGBA image.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn render(
        &self,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
        self.validate_character_context(character)?;
        self.render_offscreen(character, camera, width, height)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn render_offscreen(
        &self,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
        let target = self.acquire_offscreen_target(width, height)?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.encode_render_pass(
            &mut encoder,
            &target.texture_view,
            &target.depth_view,
            &self.pipeline,
            &self.nametag_pipeline,
            &self.nametag_see_through_pipeline,
            character,
            camera,
            width,
            height,
        );

        readback::copy_render_target_to_buffer(
            &mut encoder,
            &target.render_texture,
            &target.staging_buffer,
            width,
            height,
            target.padded_bytes_per_row,
        );

        self.queue.submit(Some(encoder.finish()));

        let image = readback::map_output_buffer_to_rgba(
            &self.device,
            &target.staging_buffer,
            width,
            height,
            target.padded_bytes_per_row,
        );

        self.release_offscreen_target(target);

        image
    }

    /// Render to an offscreen texture and return an RGBA [`image::ImageBuffer`] (async readback).
    ///
    /// Available on WASM where blocking buffer-map is not possible.
    /// Uses [`wasm_bindgen_futures`] under the hood; callers must execute within a WASM async runtime.
    #[cfg(target_arch = "wasm32")]
    pub async fn render_async(
        &self,
        character: &Character,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
        self.validate_character_context(character)?;
        let target = self.acquire_offscreen_target(width, height)?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.encode_render_pass(
            &mut encoder,
            &target.texture_view,
            &target.depth_view,
            &self.pipeline,
            &self.nametag_pipeline,
            &self.nametag_see_through_pipeline,
            character,
            camera,
            width,
            height,
        );

        readback::copy_render_target_to_buffer(
            &mut encoder,
            &target.render_texture,
            &target.staging_buffer,
            width,
            height,
            target.padded_bytes_per_row,
        );

        self.queue.submit(Some(encoder.finish()));

        let image = readback::map_output_buffer_to_rgba(
            &self.device,
            &target.staging_buffer,
            width,
            height,
            target.padded_bytes_per_row,
        )
        .await;

        self.release_offscreen_target(target);

        image
    }

    /// Present one frame to the window surface (expects `new_windowed`).
    pub fn render_frame(&self, character: &Character, camera: &Camera) -> Result<(), EidolonError> {
        self.validate_character_context(character)?;
        let surface = self.surface.as_ref().expect("Not in windowed mode");
        let config = self.surface_config.as_ref().unwrap();

        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Timeout => {
                return Err(crate::error::SurfaceError::Timeout.into())
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(crate::error::SurfaceError::Occluded.into())
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                return Err(crate::error::SurfaceError::Outdated.into())
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Err(crate::error::SurfaceError::Lost.into())
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(crate::error::SurfaceError::Validation.into())
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (_depth_texture, depth_view) =
            self.get_or_create_windowed_depth(config.width, config.height);

        let pipeline = self.surface_pipeline.as_ref().unwrap_or(&self.pipeline);
        let nametag_pipeline = self
            .nametag_surface_pipeline
            .as_ref()
            .unwrap_or(&self.nametag_pipeline);
        let nametag_see_through_pipeline = self
            .nametag_see_through_surface_pipeline
            .as_ref()
            .unwrap_or(&self.nametag_see_through_pipeline);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Frame Encoder"),
            });

        self.encode_render_pass(
            &mut encoder,
            &view,
            &depth_view,
            pipeline,
            nametag_pipeline,
            nametag_see_through_pipeline,
            character,
            camera,
            config.width,
            config.height,
        );

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }

    /// Set the render pass clear color (RGBA, 0.0–1.0). Default is transparent black.
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.clear_color = [r, g, b, a];
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Renderer>();
    }

    #[test]
    fn renderer_concurrent_render_different_threads() {
        let renderer = Arc::new(Renderer::new().expect("Failed to create renderer"));
        let body = renderer
            .load_body("resources/bingling_sama.png", SkinType::Classic)
            .expect("Failed to load body");
        let character = Arc::new(Character::from_body(body));

        let handles: Vec<_> = (0..3)
            .map(|i| {
                let r = Arc::clone(&renderer);
                let c = Arc::clone(&character);
                std::thread::spawn(move || {
                    let camera = Camera {
                        yaw: 180.0 + (i as f32) * 10.0,
                        pitch: 90.0,
                        scale: 1.0,
                    };
                    let image = r.render(&c, &camera, 64, 64).expect("render failed");
                    assert_eq!(image.width(), 64);
                    assert_eq!(image.height(), 64);
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }
    }
}
