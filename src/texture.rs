//! Skin texture loading for GPU rendering.
//!
//! Loads a PNG from disk, uploads RGBA8 data to a `wgpu` texture, and builds a bind group.
//! If the image has single-layer layout (width = 2 × height), it is converted to a
//! double-layer skin via [`crate::utils::converter::single2double`] before upload.

use crate::utils::converter::single2double;
use image::{DynamicImage, GenericImageView, ImageFormat};
use log::info;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// GPU skin texture: [`wgpu::Texture`], view, and bind group for the skin shader.
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

impl Texture {
    /// Load a skin from a PNG file path, decode as RGBA, optionally convert single-layer skins,
    /// then create the GPU texture and bind group.
    ///
    /// The path is canonicalized before use to resolve symlinks and `..` components.
    /// Returns an error if the path contains null bytes or cannot be resolved.
    pub fn load_from_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Loading texture: {}", path);
        if path.contains('\0') {
            return Err("Texture path contains null bytes".into());
        }
        let canonical = Path::new(path).canonicalize().map_err(|e| {
            format!("Failed to resolve texture path '{}': {}", path, e)
        })?;
        let file = File::open(&canonical)?;
        let reader = BufReader::new(file);
        let image = image::load(reader, ImageFormat::Png)?.to_rgba8();
        Self::load_texture(
            device,
            queue,
            bind_group_layout,
            sampler,
            &DynamicImage::ImageRgba8(image),
        )
    }

    fn load_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        image: &DynamicImage,
    ) -> Result<Texture, Box<dyn std::error::Error>> {
        let (width, height) = image.dimensions();
        info!("Texture dimensions: {}x{}", width, height);

        let image = if width == height * 2 {
            info!("Single-layer skin detected, converting to double-layer...");
            match single2double(image) {
                Ok(img) => img,
                Err(e) => {
                    return Err(
                        format!("Failed to convert single-layer to double-layer: {}", e).into(),
                    )
                }
            }
        } else {
            image.clone()
        };

        let image_dimensions = image.dimensions();
        let image_rgba = image.to_rgba8();
        let size = wgpu::Extent3d {
            width: image_dimensions.0,
            height: image_dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Skin Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image_dimensions.0),
                rows_per_image: Some(image_dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skin Texture Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        info!("Texture loaded into GPU");
        Ok(Texture {
            texture,
            view,
            bind_group,
        })
    }
}
