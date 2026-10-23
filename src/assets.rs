//! CPU-side asset representations for skins and back equipment (capes/elytra).
//!
//! Separates file I/O, image decoding, validation, and layout transformations
//! from GPU resource allocation.

use crate::error::EidolonError;
#[cfg(not(target_arch = "wasm32"))]
use image::ImageFormat;
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

// 64×32 right leg: source rects on the upper half (OFIB face order) in pixel coords (x0, y0, x1, y1).
const RIGHT_LEG_OUTSIDE_RANGE: (u32, u32, u32, u32) = (0, 20, 4, 32);
const RIGHT_LEG_TOP_FRONT_RANGE: (u32, u32, u32, u32) = (4, 16, 8, 32);
const RIGHT_LEG_BUTTOM_RANGE: (u32, u32, u32, u32) = (8, 16, 12, 20);
const RIGHT_LEG_INSIDE_RANGE: (u32, u32, u32, u32) = (8, 20, 12, 32);
const RIGHT_LEG_BACK_RANGE: (u32, u32, u32, u32) = (12, 20, 16, 32);

// 64×64 double-layer layout: left leg destination rects on the lower half (mirrored layout).
const LEFT_LEG_OUTSIDE_RANGE: (u32, u32, u32, u32) = (24, 52, 28, 64);
const LEFT_LEG_TOP_FRONT_RANGE: (u32, u32, u32, u32) = (20, 48, 24, 64);
const LEFT_LEG_BUTTOM_RANGE: (u32, u32, u32, u32) = (24, 48, 28, 52);
const LEFT_LEG_INSIDE_RANGE: (u32, u32, u32, u32) = (16, 52, 20, 64);
const LEFT_LEG_BACK_RANGE: (u32, u32, u32, u32) = (28, 52, 32, 64);

// Right arm source rects on the upper half.
const RIGHT_ARM_OUTSIDE_RANGE: (u32, u32, u32, u32) = (40, 20, 44, 32);
const RIGHT_ARM_TOP_RANGE: (u32, u32, u32, u32) = (44, 16, 48, 20);
const RIGHT_ARM_FRONT_RANGE: (u32, u32, u32, u32) = (44, 20, 48, 32);
const RIGHT_ARM_BUTTOM_RANGE: (u32, u32, u32, u32) = (48, 16, 52, 20);
const RIGHT_ARM_INSIDE_RANGE: (u32, u32, u32, u32) = (48, 20, 52, 32);
const RIGHT_ARM_BACK_RANGE: (u32, u32, u32, u32) = (52, 20, 56, 32);

// Left arm destination rects on the lower half.
const LEFT_ARM_INSIDE_RANGE: (u32, u32, u32, u32) = (32, 52, 36, 64);
const LEFT_ARM_TOP_RANGE: (u32, u32, u32, u32) = (36, 48, 40, 52);
const LEFT_ARM_FRONT_RANGE: (u32, u32, u32, u32) = (36, 52, 40, 64);
const LEFT_ARM_BUTTOM_RANGE: (u32, u32, u32, u32) = (40, 48, 44, 52);
const LEFT_ARM_OUTSIDE_RANGE: (u32, u32, u32, u32) = (40, 52, 44, 64);
const LEFT_ARM_BACK_RANGE: (u32, u32, u32, u32) = (44, 52, 48, 64);

fn scale_rect(rect: (u32, u32, u32, u32), hd_ratio: f32) -> (u32, u32, u32, u32) {
    (
        (rect.0 as f32 * hd_ratio) as u32,
        (rect.1 as f32 * hd_ratio) as u32,
        (rect.2 as f32 * hd_ratio) as u32,
        (rect.3 as f32 * hd_ratio) as u32,
    )
}

/// Expand a legacy single-layer skin (`width == 2 * height`) to a square double-layer atlas by
/// copying the top half and synthesizing mirrored left limbs in the bottom half.
pub fn single2double(img: &DynamicImage) -> Result<DynamicImage, EidolonError> {
    if img.width() != img.height() * 2 {
        return Err(EidolonError::conversion(
            "Input image is not a single-layer skin (width must be twice the height)",
        ));
    }

    let hd_ratio = img.width() as f32 / 64.0;
    let scale = |rect: (u32, u32, u32, u32)| scale_rect(rect, hd_ratio);

    let mut output_img = ImageBuffer::new(img.width(), img.width());

    for y in 0..img.height() {
        for x in 0..img.width() {
            let pixel = img.get_pixel(x, y);
            output_img.put_pixel(x, y, pixel);
        }
    }

    const REGION_PAIRS: &[((u32, u32, u32, u32), (u32, u32, u32, u32))] = &[
        // Right leg parts → left leg positions
        (RIGHT_LEG_OUTSIDE_RANGE, LEFT_LEG_OUTSIDE_RANGE),
        (RIGHT_LEG_TOP_FRONT_RANGE, LEFT_LEG_TOP_FRONT_RANGE),
        (RIGHT_LEG_BUTTOM_RANGE, LEFT_LEG_BUTTOM_RANGE),
        (RIGHT_LEG_INSIDE_RANGE, LEFT_LEG_INSIDE_RANGE),
        (RIGHT_LEG_BACK_RANGE, LEFT_LEG_BACK_RANGE),
        // Right arm parts → left arm positions
        (RIGHT_ARM_OUTSIDE_RANGE, LEFT_ARM_OUTSIDE_RANGE),
        (RIGHT_ARM_TOP_RANGE, LEFT_ARM_TOP_RANGE),
        (RIGHT_ARM_FRONT_RANGE, LEFT_ARM_FRONT_RANGE),
        (RIGHT_ARM_BUTTOM_RANGE, LEFT_ARM_BUTTOM_RANGE),
        (RIGHT_ARM_INSIDE_RANGE, LEFT_ARM_INSIDE_RANGE),
        (RIGHT_ARM_BACK_RANGE, LEFT_ARM_BACK_RANGE),
    ];

    for (src_rect, dst_rect) in REGION_PAIRS {
        let src = scale(*src_rect);
        let dst = scale(*dst_rect);
        let cropped = img
            .view(src.0, src.1, src.2 - src.0, src.3 - src.1)
            .to_image();
        let flipped = imageops::flip_horizontal(&cropped);
        imageops::overlay(&mut output_img, &flipped, dst.0 as i64, dst.1 as i64);
    }

    Ok(DynamicImage::ImageRgba8(output_img))
}

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

    /// Convert a single-layer skin image to a double-layer image.
    ///
    /// If the input image is single-layer (width == 2 * height), returns a new
    /// double-layer square image with mirrored left limbs.
    pub fn convert_single_to_double_layer(image: &DynamicImage) -> Result<DynamicImage, EidolonError> {
        single2double(image)
    }

    /// Convert a single-layer skin file on disk to a double-layer skin and write it to `output`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn convert_file(
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> Result<(), EidolonError> {
        let skin = Self::from_file(input)?;
        skin.save_to_file(output)
    }

    /// Save this skin as an image to disk.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), EidolonError> {
        let path = path.as_ref();
        self.image.save(path).map_err(|e| {
            EidolonError::output(format!(
                "failed to save skin to '{}': {}",
                path.display(),
                e
            ))
        })
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
    fn test_single2double_success_synthetic() {
        let img = DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(
            64,
            32,
            Rgba([255, 0, 0, 255]),
        ));
        let result = single2double(&img);
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 64);
    }

    #[test]
    fn test_single2double_invalid_aspect_ratio() {
        let img =
            DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(64, 31, Rgba([0, 0, 0, 255])));
        assert!(single2double(&img).is_err());

        let img2 =
            DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(64, 64, Rgba([0, 0, 0, 255])));
        assert!(single2double(&img2).is_err());
    }

    #[test]
    fn raw_cape_validates_dimensions() {
        let img = DynamicImage::new_rgba8(64, 32);
        assert!(RawCapeAtlas::from_image(&img).is_ok());

        let bad_img = DynamicImage::new_rgba8(64, 64);
        assert!(RawCapeAtlas::from_image(&bad_img).is_err());
    }
}
