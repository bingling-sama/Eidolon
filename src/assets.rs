//! CPU-side asset representations for skins and back equipment (capes/elytra).
//!
//! Separates file I/O, image decoding, validation, and layout transformations
//! from GPU resource allocation.

use crate::converter::single2double;
use crate::error::EidolonError;
use image::{DynamicImage, GenericImageView, ImageBuffer, ImageFormat, Rgba, RgbaImage};
use std::path::Path;

/// CPU-side validated skin image data ready for GPU upload.
#[derive(Clone, Debug, PartialEq)]
pub struct RawSkin {
    pub(crate) image: RgbaImage,
}

impl RawSkin {
    /// Create a RawSkin from an RGBA image buffer.
    ///
    /// If the image is single-layer (width == 2 * height), it is automatically
    /// converted to double-layer (square).
    pub fn from_image(image: &DynamicImage) -> Result<Self, EidolonError> {
        let (width, height) = image.dimensions();
        let converted = if width == height * 2 {
            single2double(image)?
        } else {
            image.clone()
        };
        Ok(Self {
            image: converted.to_rgba8(),
        })
    }

    /// Load and decode a skin from memory bytes (e.g. PNG).
    pub fn from_memory(bytes: &[u8]) -> Result<Self, EidolonError> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(&img)
    }

    /// Load and decode a skin from a file path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, EidolonError> {
        let path = path.as_ref();
        let path_str = path.to_str().unwrap_or_default();
        if path_str.contains('\0') {
            return Err(EidolonError::invalid_path("skin path contains null bytes"));
        }
        let canonical = path.canonicalize().map_err(|e| {
            EidolonError::invalid_path(format!("failed to resolve '{}': {}", path.display(), e))
        })?;
        let file = std::fs::File::open(&canonical)?;
        let reader = std::io::BufReader::new(file);
        let img = image::load(reader, ImageFormat::Png)?;
        Self::from_image(&img)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    pub fn as_raw(&self) -> &[u8] {
        self.image.as_raw()
    }

    pub fn into_inner(self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        self.image
    }
}

/// CPU-side validated back equipment atlas (cape / elytra) ready for GPU upload.
#[derive(Clone, Debug, PartialEq)]
pub struct RawCapeAtlas {
    pub(crate) image: RgbaImage,
}

impl RawCapeAtlas {
    /// Create a RawCapeAtlas from an image, validating that dimensions are 64x32 or an integer HD multiple.
    pub fn from_image(image: &DynamicImage) -> Result<Self, EidolonError> {
        let (width, height) = image.dimensions();
        validate_cape_atlas_dimensions(width, height)?;
        Ok(Self {
            image: image.to_rgba8(),
        })
    }

    /// Load and decode a cape atlas from memory bytes.
    pub fn from_memory(bytes: &[u8]) -> Result<Self, EidolonError> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(&img)
    }

    /// Load and decode a cape atlas from a file path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, EidolonError> {
        let path = path.as_ref();
        let path_str = path.to_str().unwrap_or_default();
        if path_str.contains('\0') {
            return Err(EidolonError::invalid_path("cape path contains null bytes"));
        }
        let canonical = path.canonicalize().map_err(|e| {
            EidolonError::invalid_path(format!("failed to resolve '{}': {}", path.display(), e))
        })?;
        let file = std::fs::File::open(&canonical)?;
        let reader = std::io::BufReader::new(file);
        let img = image::load(reader, ImageFormat::Png)?;
        Self::from_image(&img)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    pub fn as_raw(&self) -> &[u8] {
        self.image.as_raw()
    }

    pub fn into_inner(self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        self.image
    }
}

pub(crate) fn validate_cape_atlas_dimensions(width: u32, height: u32) -> Result<(), EidolonError> {
    let width_scale = width / 64;
    let height_scale = height / 32;
    let is_integer_hd_atlas = width >= 64
        && height >= 32
        && width.is_multiple_of(64)
        && height.is_multiple_of(32)
        && width_scale == height_scale;
    if !is_integer_hd_atlas {
        return Err(EidolonError::texture(format!(
            "cape atlas must be 64x32 or an integer HD multiple, got {width}x{height}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_skin_auto_converts_single_layer() {
        let img = DynamicImage::new_rgba8(64, 32);
        let skin = RawSkin::from_image(&img).expect("should convert single-layer");
        assert_eq!(skin.dimensions(), (64, 64));
    }

    #[test]
    fn raw_skin_preserves_double_layer() {
        let img = DynamicImage::new_rgba8(64, 64);
        let skin = RawSkin::from_image(&img).expect("should accept double-layer");
        assert_eq!(skin.dimensions(), (64, 64));
    }

    #[test]
    fn raw_cape_validates_dimensions() {
        let img = DynamicImage::new_rgba8(64, 32);
        assert!(RawCapeAtlas::from_image(&img).is_ok());

        let bad_img = DynamicImage::new_rgba8(64, 64);
        assert!(RawCapeAtlas::from_image(&bad_img).is_err());
    }
}
