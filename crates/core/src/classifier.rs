//! Texture format classification and inspection.
//!
//! Provides inspection of raw images and PNG bytes to detect Minecraft skin
//! formats (single-layer legacy, double-layer standard/HD) and back equipment
//! textures (cape, elytra).

pub use crate::body::SkinType;
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
    /// Inferred player arm geometry model (Classic or Slim), if applicable.
    pub skin_type: Option<SkinType>,
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
        let skin_type = infer_skin_type(image);
        let model_label = match skin_type {
            Some(SkinType::Slim) => " (slim arm)",
            Some(SkinType::Classic) => " (classic arm)",
            None => "",
        };
        let desc = if is_hd {
            format!(
                "HD {}x{} double-layer skin ({}x){}",
                width, height, hd_scale, model_label
            )
        } else {
            format!("Standard 64x64 double-layer skin{}", model_label)
        };
        return TextureReport {
            kind: TextureKind::SkinDoubleLayer,
            width,
            height,
            hd_scale,
            is_hd,
            can_auto_upgrade: false,
            skin_type,
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
        let (desc, can_auto_upgrade, skin_type) = match kind {
            TextureKind::SkinSingleLayer => (
                if is_hd {
                    format!(
                        "HD {}x{} legacy single-layer skin ({}x) (classic arm)",
                        width, height, hd_scale
                    )
                } else {
                    "Legacy 64x32 single-layer skin (classic arm)".to_string()
                },
                true,
                Some(SkinType::Classic),
            ),
            TextureKind::Cape => (
                if is_hd {
                    format!("HD {}x{} cape atlas ({}x)", width, height, hd_scale)
                } else {
                    "Standard 64x32 cape atlas".to_string()
                },
                false,
                None,
            ),
            TextureKind::Elytra => (
                if is_hd {
                    format!("HD {}x{} elytra atlas ({}x)", width, height, hd_scale)
                } else {
                    "Standard 64x32 elytra atlas".to_string()
                },
                false,
                None,
            ),
            _ => ("Unrecognized 2:1 texture".to_string(), false, None),
        };

        return TextureReport {
            kind,
            width,
            height,
            hd_scale,
            is_hd,
            can_auto_upgrade,
            skin_type,
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
        skin_type: None,
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

/// Heuristically infers the player skin arm model (`Classic` 4px vs `Slim` 3px / Steve vs Alex).
///
/// Returns:
/// - `Some(SkinType::Classic)`: for legacy 2:1 skins, or double-layer skins with active/textured classic arm pixels.
/// - `Some(SkinType::Slim)`: for double-layer skins where Alex-unused arm columns are transparent or filled with placeholder markers.
/// - `None`: for non-skin textures (capes, elytras, non-standard aspect ratios, etc.).
pub fn infer_skin_type(image: &DynamicImage) -> Option<SkinType> {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || width % 64 != 0 {
        return None;
    }

    // 1. Legacy 2:1 single-layer skins (pre-1.8) only ever had 4px Steve arms
    if width == height * 2 {
        let hd_scale = width / 64;
        if classify_2_to_1_texture(image, hd_scale) == TextureKind::SkinSingleLayer {
            return Some(SkinType::Classic);
        }
        return None;
    }

    if width != height {
        return None;
    }

    let scale = width / 64;

    // 2. Square double-layer skins:
    // Classic Steve arm has width 4 (perimeter 4+4+4+4 = 16px).
    // Slim Alex arm has width 3 (perimeter 4+3+4+3 = 14px).
    // In Alex skins, the rightmost 2 columns of each arm texture wrap are unused:
    // - Right arm base layer unused zone: x in [54..56], y in [20..32] (24 base px)
    // - Left arm base layer unused zone:  x in [46..48], y in [52..64] (24 base px)
    // - Right arm overlay unused zone:    x in [54..56], y in [36..48] (24 base px)
    // - Left arm overlay unused zone:     x in [62..64], y in [52..64] (24 base px)
    let r_base = analyze_region_stats(image, 54 * scale, 20 * scale, 56 * scale, 32 * scale);
    let l_base = analyze_region_stats(image, 46 * scale, 52 * scale, 48 * scale, 64 * scale);

    // Rule 1: Synthetic placeholder / marker fill detection.
    // Some skins or test fixtures fill unused zones with a solid dummy marker (e.g. solid red)
    // rather than transparency. If variance is near 0 and color sharply differs from adjacent arm, it's Slim.
    if is_marker_fill(image, scale, 54, 53, 20, 32, &r_base)
        || is_marker_fill(image, scale, 46, 45, 52, 64, &l_base)
    {
        return Some(SkinType::Slim);
    }

    // Rule 2: Active classic pixels check.
    // If either the right or left arm unused column contains significant opaque textured pixels,
    // this skin is designed for the 4px Classic arm geometry (otherwise these pixels would be discarded).
    let r_has_classic = r_base.opaque_count > (r_base.total_pixels / 4);
    let l_has_classic = l_base.opaque_count > (l_base.total_pixels / 4);

    if r_has_classic || l_has_classic {
        return Some(SkinType::Classic);
    }

    // Rule 3: Overlay layer validation.
    // If either overlay layer has pixels in the unused strip, it's Classic.
    let r_over = analyze_region_stats(image, 54 * scale, 36 * scale, 56 * scale, 48 * scale);
    let l_over = analyze_region_stats(image, 62 * scale, 52 * scale, 64 * scale, 64 * scale);
    if r_over.opaque_count > (2 * scale * scale) || l_over.opaque_count > (2 * scale * scale) {
        return Some(SkinType::Classic);
    }

    // Rule 4: If unused arm strips are predominantly transparent, it's Slim.
    let r_trans_ratio = r_base.transparent_count as f32 / r_base.total_pixels as f32;
    let l_trans_ratio = l_base.transparent_count as f32 / l_base.total_pixels as f32;

    if r_trans_ratio > 0.75 && l_trans_ratio > 0.75 {
        return Some(SkinType::Slim);
    }

    // Default fallback: Classic
    Some(SkinType::Classic)
}

/// Infer skin arm model from raw image bytes.
pub fn infer_skin_type_from_bytes(bytes: &[u8]) -> Result<Option<SkinType>, EidolonError> {
    let img = image::load_from_memory(bytes)?;
    Ok(infer_skin_type(&img))
}

#[derive(Debug, Clone, Copy)]
struct RegionStats {
    total_pixels: u32,
    transparent_count: u32,
    opaque_count: u32,
    unique_colors: usize,
}

fn analyze_region_stats(image: &DynamicImage, x0: u32, y0: u32, x1: u32, y1: u32) -> RegionStats {
    let mut total_pixels = 0;
    let mut transparent_count = 0;
    let mut opaque_count = 0;
    let mut colors = std::collections::HashSet::new();

    let (w, h) = image.dimensions();
    let x_end = x1.min(w);
    let y_end = y1.min(h);

    for y in y0.min(h)..y_end {
        for x in x0.min(w)..x_end {
            total_pixels += 1;
            let p = image.get_pixel(x, y);
            if p[3] <= 10 {
                transparent_count += 1;
            } else {
                opaque_count += 1;
                if colors.len() <= 8 {
                    colors.insert(p);
                }
            }
        }
    }

    RegionStats {
        total_pixels,
        transparent_count,
        opaque_count,
        unique_colors: colors.len(),
    }
}

fn is_marker_fill(
    image: &DynamicImage,
    scale: u32,
    unused_col_x: u32,
    adj_col_x: u32,
    y0: u32,
    y1: u32,
    stats: &RegionStats,
) -> bool {
    if stats.opaque_count > 0 && stats.unique_colors <= 2 {
        let mut total_diff: u64 = 0;
        let mut count: u64 = 0;

        for y in (y0 * scale)..(y1 * scale) {
            let p_adj = image.get_pixel(adj_col_x * scale, y);
            let p_unused = image.get_pixel(unused_col_x * scale, y);
            let diff = (p_adj[0] as i32 - p_unused[0] as i32).abs() as u64
                + (p_adj[1] as i32 - p_unused[1] as i32).abs() as u64
                + (p_adj[2] as i32 - p_unused[2] as i32).abs() as u64;
            total_diff += diff;
            count += 1;
        }

        if count > 0 && (total_diff / count) > 100 {
            return true;
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
        assert_eq!(rep.skin_type, None);
    }

    #[test]
    fn test_infer_skin_type_bingling_sama() {
        let bytes = std::fs::read(crate::test_support::resource("bingling_sama.png")).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!(infer_skin_type(&img), Some(SkinType::Slim));

        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::SkinDoubleLayer);
        assert_eq!(rep.skin_type, Some(SkinType::Slim));
    }

    #[test]
    fn test_infer_skin_type_undefined_r2() {
        let bytes = std::fs::read(crate::test_support::resource("undefinedR2.png")).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!(infer_skin_type(&img), Some(SkinType::Classic));

        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::SkinDoubleLayer);
        assert_eq!(rep.skin_type, Some(SkinType::Classic));
    }

    #[test]
    fn test_infer_skin_type_legacy_steven() {
        let bytes = std::fs::read(crate::test_support::resource("SSSSSteven.png")).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!(infer_skin_type(&img), Some(SkinType::Classic));

        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::SkinSingleLayer);
        assert_eq!(rep.skin_type, Some(SkinType::Classic));
    }

    #[test]
    fn test_infer_skin_type_turtle_cape() {
        let bytes = std::fs::read(crate::test_support::resource("Turtle_Cape.png")).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!(infer_skin_type(&img), None);

        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::Cape);
        assert_eq!(rep.skin_type, None);
    }

    #[test]
    fn test_infer_skin_type_synthetic_transparent_alex() {
        let mut buf = ImageBuffer::new(64, 64);
        // Paint torso and head fully opaque
        for y in 0..32 {
            for x in 0..40 {
                buf.put_pixel(x, y, Rgba([120, 100, 80, 255]));
            }
        }
        // Paint right arm active columns (40..54), leaving 54..56 transparent
        for y in 20..32 {
            for x in 40..54 {
                buf.put_pixel(x, y, Rgba([200, 150, 100, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        assert_eq!(infer_skin_type(&img), Some(SkinType::Slim));
    }

    #[test]
    fn test_infer_skin_type_synthetic_hd_transparent_alex() {
        let mut buf = ImageBuffer::new(128, 128);
        let scale = 2;
        // Paint right arm active columns (40*2..54*2), leaving 54*2..56*2 transparent
        for y in (20 * scale)..(32 * scale) {
            for x in (40 * scale)..(54 * scale) {
                buf.put_pixel(x, y, Rgba([200, 150, 100, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        assert_eq!(infer_skin_type(&img), Some(SkinType::Slim));
    }

    #[test]
    fn test_infer_skin_type_synthetic_classic_steve() {
        let mut buf = ImageBuffer::new(64, 64);
        // Paint right arm with varying texture across all columns 40..56
        for y in 20..32 {
            for x in 40..56 {
                let r = (x * 10) as u8;
                let g = (y * 5) as u8;
                buf.put_pixel(x, y, Rgba([r, g, 100, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        assert_eq!(infer_skin_type(&img), Some(SkinType::Classic));
    }
}
