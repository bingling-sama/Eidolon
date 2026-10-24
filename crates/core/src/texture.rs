//! Skin texture loading for GPU rendering.
//!
//! Loads a PNG from disk, uploads RGBA8 data to a `wgpu` texture, and builds a bind group.
//! If the image has single-layer layout (width = 2 × height), it is converted to a
//! double-layer skin via [`crate::converter::single2double`] before upload.

use crate::error::EidolonError;
use log::info;

/// GPU image atlas.
///
/// Wraps wgpu resources; consumed by the renderer via its bind group.
/// Fields are private — no wgpu types leak to library consumers.
pub struct Texture {
    #[allow(dead_code)]
    pub(crate) texture: wgpu::Texture,
    #[allow(dead_code)]
    pub(crate) view: wgpu::TextureView,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl Texture {
    pub(crate) fn upload_raw(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Texture, EidolonError> {
        let size = wgpu::Extent3d {
            width,
            height,
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
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
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

#[cfg(test)]
mod tests {
    use crate::assets::validate_cape_atlas_dimensions;

    #[test]
    fn cape_atlas_accepts_base_and_hd_dimensions() {
        assert!(validate_cape_atlas_dimensions(64, 32).is_ok());
        assert!(validate_cape_atlas_dimensions(128, 64).is_ok());
        assert!(validate_cape_atlas_dimensions(256, 128).is_ok());
    }

    #[test]
    fn cape_atlas_rejects_non_integer_scales_and_wrong_aspects() {
        assert!(validate_cape_atlas_dimensions(2, 1).is_err());
        assert!(validate_cape_atlas_dimensions(100, 50).is_err());
        assert!(validate_cape_atlas_dimensions(64, 64).is_err());
        assert!(validate_cape_atlas_dimensions(128, 32).is_err());
    }
}
