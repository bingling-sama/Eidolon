//! 纹理模块
//!
//! 这个模块负责加载和处理 Minecraft 皮肤纹理。
//! 支持从 PNG 文件加载纹理，并创建 wgpu 纹理对象。

use crate::utils::converter::single2double;
use image::{DynamicImage, GenericImageView, ImageFormat};
use log::info;
use std::fs::File;
use std::io::BufReader;

/// 纹理结构体
///
/// 封装了 wgpu 纹理对象及其视图和绑定组，提供了纹理加载和管理功能。
/// 支持从文件加载 PNG 格式的纹理。
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

impl Texture {
    /// 从文件加载纹理
    ///
    /// 从指定路径加载 PNG 格式的纹理文件，创建 wgpu 纹理对象。
    /// 支持 RGBA 格式的图像，自动处理图像格式转换。
    ///
    /// # 参数
    ///
    /// * `device` - wgpu 设备
    /// * `queue` - wgpu 命令队列
    /// * `bind_group_layout` - 纹理绑定组布局
    /// * `sampler` - 纹理采样器
    /// * `path` - 纹理文件路径
    pub fn load_from_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Loading texture: {}", path);
        let file = File::open(path)?;
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
