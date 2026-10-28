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
    let trans_ratio = measure_transparency_ratio(image, scale);

    // Grid-based sampling in base 64x32 coordinates (sampling center of each base texel)
    // 1. Lower body: x in [0..56], y in [24..32] (legs, lower torso, lower right arm in skin)
    let (lower_opaque, _lower_total) = sample_grid_opaque_count(image, scale, 0, 24, 56, 32);

    // 2. Right arm exclusive: x in [48..56], y in [22..32]
    // In Minecraft elytra textures, the wing reaches x up to ~46-47 and y up to ~22.
    // [48..56, 22..32] is exclusively right arm / skin territory.
    let (arm_opaque, _arm_total) = sample_grid_opaque_count(image, scale, 48, 22, 56, 32);

    // 3. Cape main body: x in [1..21], y in [1..16]
    let (cape_opaque, cape_total) = sample_grid_opaque_count(image, scale, 1, 1, 21, 16);
    let cape_density = if cape_total > 0 {
        cape_opaque as f32 / cape_total as f32
    } else {
        0.0
    };

    // 4. Elytra wing main region: x in [23..45], y in [2..21]
    let (elytra_opaque, elytra_total) = sample_grid_opaque_count(image, scale, 23, 2, 45, 21);
    let elytra_density = if elytra_total > 0 {
        elytra_opaque as f32 / elytra_total as f32
    } else {
        0.0
    };

    // Case 1: Standard textures with transparency (trans_ratio > 0.15)
    if trans_ratio > 0.15 {
        if lower_opaque > 0 || arm_opaque > 0 {
            return TextureKind::SkinSingleLayer;
        }
        if cape_density > 0.05 && elytra_density > 0.05 {
            // Modern Minecraft cape atlas contains BOTH cape texture at [0..22, 0..17]
            // AND matching elytra texture at [22..46, 0..22] (e.g. Turtle Cape).
            return TextureKind::Cape;
        }
        if elytra_density > 0.05 {
            return TextureKind::Elytra;
        }
        if cape_density > 0.05 {
            return TextureKind::Cape;
        }
        return TextureKind::Unknown;
    }

    // Case 2: Fully opaque or near-opaque textures (trans_ratio <= 0.15)
    // Legacy skins have human facial features with high contrast/variance across [8..16, 8..16].
    // Solid, chroma-keyed, or continuous gradient capes lack skin face features (stdev < 15.0).
    let stdev = face_luminance_stdev(image, scale);
    if stdev < 15.0 {
        return TextureKind::Cape;
    }

    TextureKind::SkinSingleLayer
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
    //
    // Check 1: Overlay layer validation (strong Slim signal when sleeves are cleanly tailored).
    // If the skin wears sleeve overlays (outer layer active), Alex skins strictly avoid
    // the 4th column strip ([54..56, 36..48] for right arm, [62..64, 48..64] for left arm).
    // Note: We do NOT immediately conclude Classic here if there are stray pixels,
    // because artists or skin editors often leave stray brushstrokes/cuffs in outer overlays.
    let (r_over_act, _) = sample_grid_opaque_count(image, scale, 44, 36, 53, 48);
    let (r_over_un, r_over_tot) = sample_grid_opaque_count(image, scale, 54, 36, 56, 48);
    let (l_over_act, _) = sample_grid_opaque_count(image, scale, 52, 48, 61, 64);
    let (l_over_un, l_over_tot) = sample_grid_opaque_count(image, scale, 62, 48, 64, 64);

    if r_over_act > 4 || l_over_act > 4 {
        let r_ratio = if r_over_tot > 0 {
            r_over_un as f32 / r_over_tot as f32
        } else {
            0.0
        };
        let l_ratio = if l_over_tot > 0 {
            l_over_un as f32 / l_over_tot as f32
        } else {
            0.0
        };

        if r_ratio < 0.05 && l_ratio < 0.05 {
            return Some(SkinType::Slim);
        }
    }

    // Check 2: Adjacent column discontinuity / dummy marker fill in base layer.
    // In Alex skins where the base layer unused strip is filled (solid placeholder like pure black/red,
    // or flat skin tone), there is a sharp color discontinuity between the arm edge (x=53 on right arm,
    // x=45 on left arm) and the unused strip (x=54 on right arm, x=46 on left arm).
    let diff_r = avg_column_color_diff(image, scale, 53, 54, 20, 32);
    let diff_l = avg_column_color_diff(image, scale, 45, 46, 52, 64);
    if diff_r > 70.0 || diff_l > 70.0 {
        return Some(SkinType::Slim);
    }

    // Check 3: Base layer transparency in unused zones:
    // - Right arm base unused: [54..56, 20..32], active: [40..54, 20..32]
    // - Left arm base unused:  [46..48, 52..64], active: [32..46, 52..64]
    let (r_base_act, _) = sample_grid_opaque_count(image, scale, 40, 20, 54, 32);
    let (r_base_un, r_base_tot) = sample_grid_opaque_count(image, scale, 54, 20, 56, 32);
    let (l_base_act, _) = sample_grid_opaque_count(image, scale, 32, 52, 46, 64);
    let (l_base_un, l_base_tot) = sample_grid_opaque_count(image, scale, 46, 52, 48, 64);

    let r_has_arm = r_base_act > 0 || r_base_un > 0;
    let l_has_arm = l_base_act > 0 || l_base_un > 0;

    let r_trans = if r_base_tot > 0 {
        1.0 - (r_base_un as f32 / r_base_tot as f32)
    } else {
        0.0
    };
    let l_trans = if l_base_tot > 0 {
        1.0 - (l_base_un as f32 / l_base_tot as f32)
    } else {
        0.0
    };

    if (r_has_arm && l_has_arm && r_trans > 0.70 && l_trans > 0.70)
        || (r_has_arm && !l_has_arm && r_trans > 0.70)
        || (!r_has_arm && l_has_arm && l_trans > 0.70)
    {
        return Some(SkinType::Slim);
    }

    // Fallback: Classic Steve
    Some(SkinType::Classic)
}

/// Infer skin arm model from raw image bytes.
pub fn infer_skin_type_from_bytes(bytes: &[u8]) -> Result<Option<SkinType>, EidolonError> {
    let img = image::load_from_memory(bytes)?;
    Ok(infer_skin_type(&img))
}

fn sample_grid_opaque_count(
    image: &DynamicImage,
    scale: u32,
    bx0: u32,
    by0: u32,
    bx1: u32,
    by1: u32,
) -> (u32, u32) {
    let (w, h) = image.dimensions();
    let mut opaque = 0;
    let mut total = 0;

    for by in by0..by1 {
        for bx in bx0..bx1 {
            let px = ((bx as u64 * 2 + 1) * scale as u64 / 2).min((w - 1) as u64) as u32;
            let py = ((by as u64 * 2 + 1) * scale as u64 / 2).min((h - 1) as u64) as u32;
            total += 1;
            if image.get_pixel(px, py)[3] > 10 {
                opaque += 1;
            }
        }
    }

    (opaque, total)
}

fn measure_transparency_ratio(image: &DynamicImage, scale: u32) -> f32 {
    let (width, height) = image.dimensions();
    let mut total = 0u64;
    let mut trans = 0u64;
    let step = scale.max(1);

    for y in (0..height).step_by(step as usize) {
        for x in (0..width).step_by(step as usize) {
            total += 1;
            if image.get_pixel(x, y)[3] <= 10 {
                trans += 1;
            }
        }
    }

    if total > 0 {
        trans as f32 / total as f32
    } else {
        0.0
    }
}

fn face_luminance_stdev(image: &DynamicImage, scale: u32) -> f32 {
    let (w, h) = image.dimensions();
    let mut lums = Vec::with_capacity(64);

    for by in 8..16 {
        for bx in 8..16 {
            let px = ((bx as u64 * 2 + 1) * scale as u64 / 2).min((w - 1) as u64) as u32;
            let py = ((by as u64 * 2 + 1) * scale as u64 / 2).min((h - 1) as u64) as u32;
            let p = image.get_pixel(px, py);
            let lum = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
            lums.push(lum);
        }
    }

    if lums.is_empty() {
        return 0.0;
    }

    let mean = lums.iter().sum::<f32>() / lums.len() as f32;
    let variance = lums.iter().map(|&l| (l - mean).powi(2)).sum::<f32>() / lums.len() as f32;
    variance.sqrt()
}

fn avg_column_color_diff(
    image: &DynamicImage,
    scale: u32,
    bx_a: u32,
    bx_b: u32,
    by0: u32,
    by1: u32,
) -> f32 {
    let (w, h) = image.dimensions();
    let mut total_diff: f64 = 0.0;
    let mut count: usize = 0;

    for by in by0..by1 {
        let px_a = ((bx_a as u64 * 2 + 1) * scale as u64 / 2).min((w - 1) as u64) as u32;
        let px_b = ((bx_b as u64 * 2 + 1) * scale as u64 / 2).min((w - 1) as u64) as u32;
        let py = ((by as u64 * 2 + 1) * scale as u64 / 2).min((h - 1) as u64) as u32;

        let pa = image.get_pixel(px_a, py);
        let pb = image.get_pixel(px_b, py);

        if pa[3] > 10 && pb[3] > 10 {
            let diff = (pa[0] as f64 - pb[0] as f64).abs()
                + (pa[1] as f64 - pb[1] as f64).abs()
                + (pa[2] as f64 - pb[2] as f64).abs();
            total_diff += diff;
            count += 1;
        }
    }

    if count > 0 {
        (total_diff / count as f64) as f32
    } else {
        0.0
    }
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

    #[test]
    fn test_synthetic_hd_cape_with_elytra() {
        let mut buf = ImageBuffer::new(1024, 512);
        let scale = 16;
        // Paint cape body at [1..21, 1..16]
        for y in scale..(16 * scale) {
            for x in scale..(21 * scale) {
                buf.put_pixel(x, y, Rgba([200, 50, 50, 255]));
            }
        }
        // Paint elytra wing at [23..45, 2..21]
        for y in (2 * scale)..(21 * scale) {
            for x in (23 * scale)..(45 * scale) {
                buf.put_pixel(x, y, Rgba([50, 200, 50, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::Cape);
        assert_eq!(rep.skin_type, None);
    }

    #[test]
    fn test_synthetic_cape_with_bottom_right_watermark() {
        let mut buf = ImageBuffer::new(2048, 1024);
        let scale = 32;
        // Paint cape body at [1..21, 1..16]
        for y in scale..(16 * scale) {
            for x in scale..(21 * scale) {
                buf.put_pixel(x, y, Rgba([200, 50, 50, 255]));
            }
        }
        // Paint elytra wing at [23..45, 2..21]
        for y in (2 * scale)..(21 * scale) {
            for x in (23 * scale)..(45 * scale) {
                buf.put_pixel(x, y, Rgba([50, 200, 50, 255]));
            }
        }
        // Paint small watermark in bottom-right corner [62..64, 30..32]
        for y in (30 * scale)..(32 * scale) {
            for x in (62 * scale)..(64 * scale) {
                buf.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::Cape);
        assert_eq!(rep.skin_type, None);
    }

    #[test]
    fn test_synthetic_chroma_key_opaque_cape() {
        let mut buf = ImageBuffer::new(64, 32);
        // Fill canvas with flat chroma green background
        for y in 0..32 {
            for x in 0..64 {
                buf.put_pixel(x, y, Rgba([0, 208, 22, 255]));
            }
        }
        // Paint cape front [1..11, 1..17] and back [12..22, 1..17] with subtle green gradient
        for y in 1..17 {
            for x in 1..22 {
                buf.put_pixel(x, y, Rgba([30, 215, 40, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        let rep = inspect_image(&img);
        assert_eq!(rep.kind, TextureKind::Cape);
        assert_eq!(rep.skin_type, None);
    }

    #[test]
    fn test_synthetic_alex_with_overlay_sleeves() {
        let mut buf = ImageBuffer::new(64, 64);
        // Paint base layer arm columns 40..56 (even if unused column has fill)
        for y in 20..32 {
            for x in 40..56 {
                buf.put_pixel(x, y, Rgba([220, 180, 140, 255]));
            }
        }
        // Paint overlay sleeve on 44..54 (active 3px Alex sleeve), leaving 54..56 empty
        for y in 36..48 {
            for x in 44..54 {
                buf.put_pixel(x, y, Rgba([50, 100, 200, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        assert_eq!(infer_skin_type(&img), Some(SkinType::Slim));
    }

    #[test]
    fn test_synthetic_alex_with_column_discontinuity() {
        let mut buf = ImageBuffer::new(64, 64);
        // Paint base layer right arm: columns 40..54 blue shirt, column 54..56 skin tone
        for y in 20..32 {
            for x in 40..54 {
                buf.put_pixel(x, y, Rgba([30, 60, 100, 255]));
            }
            for x in 54..56 {
                buf.put_pixel(x, y, Rgba([230, 180, 140, 255]));
            }
        }
        let img = DynamicImage::ImageRgba8(buf);
        assert_eq!(infer_skin_type(&img), Some(SkinType::Slim));
    }

    #[test]
    fn test_identify_dataset_files_if_present() {
        let test_cases = [
            ("identify/776877.png", TextureKind::Cape, None),
            ("identify/777050.png", TextureKind::Cape, None),
            ("identify/777234.png", TextureKind::Cape, None),
            ("identify/777235.png", TextureKind::Cape, None),
            ("identify/777236.png", TextureKind::Cape, None),
            ("identify/777278.png", TextureKind::Cape, None),
            ("identify/777293.png", TextureKind::Cape, None),
            (
                "identify/777364.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Slim),
            ),
            (
                "identify/776646.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Slim),
            ),
        ];

        for (rel_path, expected_kind, expected_skin_type) in test_cases {
            let full_path = crate::test_support::resource(rel_path);
            if full_path.exists() {
                let bytes = std::fs::read(&full_path).unwrap();
                let img = image::load_from_memory(&bytes).unwrap();
                let rep = inspect_image(&img);
                assert_eq!(
                    rep.kind, expected_kind,
                    "Failed kind check for {}",
                    rel_path
                );
                assert_eq!(
                    rep.skin_type, expected_skin_type,
                    "Failed skin_type check for {}",
                    rel_path
                );
            }
        }
    }
}
