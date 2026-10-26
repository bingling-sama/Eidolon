//! Texture format classification and inspection.
//!
//! Provides inspection of raw images and PNG bytes to detect Minecraft skin
//! formats (single-layer legacy, double-layer standard/HD) and back equipment
//! textures (cape, elytra).

use crate::error::EidolonError;
use image::{DynamicImage, GenericImageView};

/// Identified kind of a Minecraft texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureKind {
    /// Modern double-layer skin (square: 64x64, 128x128, etc.).
    SkinDoubleLayer,
    /// Legacy single-layer skin (2:1 aspect ratio: 64x32, 128x64, etc.).
    SkinSingleLayer,
    /// Cape texture atlas (2:1 aspect ratio: 64x32, 128x64, etc. with cape UV footprint).
    Cape,
    /// Elytra custom texture atlas (2:1 aspect ratio with elytra wing UV footprint).
    Elytra,
    /// Unrecognized dimensions or non-standard format.
    Unknown,
}

impl TextureKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SkinDoubleLayer => "skin_double_layer",
            Self::SkinSingleLayer => "skin_single_layer",
            Self::Cape => "cape",
            Self::Elytra => "elytra",
            Self::Unknown => "unknown",
        }
    }
}

/// Detailed inspection report for a texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureReport {
    /// Classified texture kind.
    pub kind: TextureKind,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// HD resolution scale (1 for standard 64x, 2 for 128x, 4 for 256x, etc.).
    pub hd_scale: u32,
    /// True if texture resolution exceeds standard 64-pixel base.
    pub is_hd: bool,
    /// True if texture is a legacy skin that can be auto-upgraded to double-layer.
    pub can_auto_upgrade: bool,
    /// Human-readable explanation.
    pub description: String,
}

/// Inspect and classify decoded image data.
pub fn inspect_image(image: &DynamicImage) -> TextureReport {
    let (width, height) = image.dimensions();

    // 1. Square dimensions (width == height) -> Skin Double Layer
    if width > 0 && width == height && width % 64 == 0 {
        let hd_scale = width / 64;
        let is_hd = hd_scale > 1;
        let desc = if is_hd {
            format!("HD {}x{} double-layer skin ({}x)", width, height, hd_scale)
        } else {
            "Standard 64x64 double-layer skin".to_string()
        };
        return TextureReport {
            kind: TextureKind::SkinDoubleLayer,
            width,
            height,
            hd_scale,
            is_hd,
            can_auto_upgrade: false,
            description: desc,
        };
    }

    // 2. 2:1 ratio (width == 2 * height) -> Could be Single-layer Skin, Cape, or Elytra
    if width > 0 && height > 0 && width == height * 2 && width % 64 == 0 {
        let hd_scale = width / 64;
        let is_hd = hd_scale > 1;

        // Disambiguation between 2:1 Cape, Elytra, and Single-layer Skin:
        // In 64x32 base coordinates:
        // - Skin has right arm/head in [44..56, 16..32] and head top/front in [0..32, 0..16].
        // - Standard cape only utilizes [0..22, 0..17] for the 10x16x1 box geometry.
        // - Standard elytra utilizes [22..46, 0..32] for wing geometry.
        let kind = classify_2_to_1_texture(image, hd_scale);
        let (desc, can_auto_upgrade) = match kind {
            TextureKind::SkinSingleLayer => (
                if is_hd {
                    format!(
                        "HD {}x{} legacy single-layer skin ({}x)",
                        width, height, hd_scale
                    )
                } else {
                    "Legacy 64x32 single-layer skin".to_string()
                },
                true,
            ),
            TextureKind::Cape => (
                if is_hd {
                    format!("HD {}x{} cape atlas ({}x)", width, height, hd_scale)
                } else {
                    "Standard 64x32 cape atlas".to_string()
                },
                false,
            ),
            TextureKind::Elytra => (
                if is_hd {
                    format!("HD {}x{} elytra atlas ({}x)", width, height, hd_scale)
                } else {
                    "Standard 64x32 elytra atlas".to_string()
                },
                false,
            ),
            _ => ("Unrecognized 2:1 texture".to_string(), false),
        };

        return TextureReport {
            kind,
            width,
            height,
            hd_scale,
            is_hd,
            can_auto_upgrade,
            description: desc,
        };
    }

    // 3. Fallback: Unknown
    TextureReport {
        kind: TextureKind::Unknown,
        width,
        height,
        hd_scale: 0,
        is_hd: false,
        can_auto_upgrade: false,
        description: format!("Non-standard texture dimensions ({}x{})", width, height),
    }
}

/// Inspect and classify texture from raw bytes (e.g. PNG).
pub fn inspect_texture(bytes: &[u8]) -> Result<TextureReport, EidolonError> {
    let img = image::load_from_memory(bytes)?;
    Ok(inspect_image(&img))
}

/// Heuristic analyzer for 2:1 textures (64x32 base).
fn classify_2_to_1_texture(image: &DynamicImage, scale: u32) -> TextureKind {
    // Check specific sampling regions (in base 64x32 coords multiplied by scale)
    // 1. Right arm & right leg lower regions (x in [46..64], y in [16..32])
    // In a 64x32 skin:
    //   Right arm is at x: 40..56, y: 16..32
    //   Right leg is at x: 0..16, y: 16..32
    //   Torso is at x: 16..40, y: 16..32
    // A skin has pixels all across y in [16..32] spanning x: 0 to 56.
    // In Minecraft official capes / elytra:
    //   Cape is at [0..22, 0..17]
    //   Elytra is at [22..46, 0..22]
    //   Area x in [46..64], y in [0..32] is COMPLETELY EMPTY in capes/elytra!
    let skin_right_arm_has_pixels =
        region_has_opaque_pixels(image, 46 * scale, 18 * scale, 64 * scale, 32 * scale);

    // 2. Elytra wing main region: x in [22..46], y in [0..22]
    let elytra_region_has_pixels =
        region_has_opaque_pixels(image, 23 * scale, 2 * scale, 45 * scale, 22 * scale);

    // 3. Cape main body: x in [0..22], y in [0..17]
    let cape_body_has_pixels =
        region_has_opaque_pixels(image, scale, scale, 21 * scale, 16 * scale);

    if skin_right_arm_has_pixels {
        TextureKind::SkinSingleLayer
    } else if elytra_region_has_pixels && cape_body_has_pixels {
        // Modern Minecraft cape atlas contains BOTH cape texture at [0..22, 0..17]
        // AND matching elytra texture at [22..46, 0..22] (e.g. Turtle Cape).
        // Since it provides both, it is the standard Back Equipment / Cape atlas!
        TextureKind::Cape
    } else if elytra_region_has_pixels {
        TextureKind::Elytra
    } else if cape_body_has_pixels {
        TextureKind::Cape
    } else {
        TextureKind::Unknown
    }
}

/// Returns true if region contains any pixel with alpha > 10.
fn region_has_opaque_pixels(image: &DynamicImage, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
    let (w, h) = image.dimensions();
    let x_end = x1.min(w);
    let y_end = y1.min(h);

    for y in y0.min(h)..y_end {
        for x in x0.min(w)..x_end {
            let p = image.get_pixel(x, y);
            if p[3] > 10 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn inspect_double_layer_skin_standard() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::new(64, 64));
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::SkinDoubleLayer);
        assert_eq!(rep.hd_scale, 1);
        assert!(!rep.is_hd);
        assert!(!rep.can_auto_upgrade);
    }

    #[test]
    fn inspect_double_layer_skin_hd() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::new(128, 128));
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::SkinDoubleLayer);
        assert_eq!(rep.hd_scale, 2);
        assert!(rep.is_hd);
    }

    #[test]
    fn inspect_single_layer_skin() {
        let mut buf = ImageBuffer::new(64, 32);
        // Paint head and arm pixels
        buf.put_pixel(10, 5, Rgba([255, 0, 0, 255]));
        buf.put_pixel(46, 25, Rgba([0, 255, 0, 255]));
        let img = DynamicImage::ImageRgba8(buf);
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::SkinSingleLayer);
        assert_eq!(rep.hd_scale, 1);
        assert!(rep.can_auto_upgrade);
    }

    #[test]
    fn analyze_turtle_cape() {
        let bytes = std::fs::read(crate::test_support::resource("Turtle_Cape.png")).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        let mut non_transparent = Vec::new();
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                if p[3] > 10 {
                    non_transparent.push((x, y, p[0], p[1], p[2], p[3]));
                }
            }
        }
        let xs: Vec<u32> = non_transparent.iter().map(|p| p.0).collect();
        let ys: Vec<u32> = non_transparent.iter().map(|p| p.1).collect();
        println!("pixels count: {}", non_transparent.len());
        println!(
            "x bounds: {}..={}",
            xs.iter().min().unwrap(),
            xs.iter().max().unwrap()
        );
        println!(
            "y bounds: {}..={}",
            ys.iter().min().unwrap(),
            ys.iter().max().unwrap()
        );
        let rep = inspect_image(&img);
        println!("rep: {:?}", rep);
        assert_eq!(rep.kind, TextureKind::Cape);
    }

    #[test]
    fn inspect_unknown_dimensions() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::new(100, 50));
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::Unknown);
    }
}
