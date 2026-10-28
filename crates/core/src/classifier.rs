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

/// Coordinates of arm regions on the standard 64x64 skin template.
///
/// In Minecraft:
/// - Steve arms are 4px wide (perimeter 4+4+4+4 = 16 texels; caps 4x4 = 16 texels).
/// - Alex arms are 3px wide (perimeter 4+3+4+3 = 14 texels; caps 3x4 = 12 texels).
/// - The remaining 2 columns in side panels and cap strips are left unused in Alex UV.
struct ArmRegion {
    bx0: u32,
    by0: u32,
    bx1: u32,
    by1: u32,
}

impl ArmRegion {
    const fn new(bx0: u32, by0: u32, bx1: u32, by1: u32) -> Self {
        Self { bx0, by0, bx1, by1 }
    }
}

// Right arm layout (base: [40..56, 16..32], overlay: [40..56, 32..48])
const RIGHT_ARM_BASE_ACTIVE_SIDES: ArmRegion = ArmRegion::new(40, 20, 54, 32);
const RIGHT_ARM_BASE_UNUSED_SIDES: ArmRegion = ArmRegion::new(54, 20, 56, 32);
const RIGHT_ARM_BASE_ACTIVE_CAP: ArmRegion = ArmRegion::new(44, 16, 50, 20);
const RIGHT_ARM_BASE_UNUSED_CAP: ArmRegion = ArmRegion::new(50, 16, 52, 20);

const RIGHT_ARM_OVER_ACTIVE_SIDES: ArmRegion = ArmRegion::new(44, 36, 54, 48);
const RIGHT_ARM_OVER_UNUSED_SIDES: ArmRegion = ArmRegion::new(54, 36, 56, 48);
const RIGHT_ARM_OVER_CAP_BOTTOM: ArmRegion = ArmRegion::new(47, 32, 50, 36);
const RIGHT_ARM_OVER_CAP_TOP: ArmRegion = ArmRegion::new(44, 32, 47, 36);

// Left arm layout (base: [32..48, 48..64], overlay: [48..64, 48..64])
const LEFT_ARM_BASE_ACTIVE_SIDES: ArmRegion = ArmRegion::new(32, 52, 46, 64);
const LEFT_ARM_BASE_UNUSED_SIDES: ArmRegion = ArmRegion::new(46, 52, 48, 64);
const LEFT_ARM_BASE_ACTIVE_CAP: ArmRegion = ArmRegion::new(36, 48, 42, 52);
const LEFT_ARM_BASE_UNUSED_CAP: ArmRegion = ArmRegion::new(42, 48, 44, 52);

const LEFT_ARM_OVER_ACTIVE_SIDES: ArmRegion = ArmRegion::new(52, 52, 62, 64);
const LEFT_ARM_OVER_UNUSED_SIDES: ArmRegion = ArmRegion::new(62, 52, 64, 64);
const LEFT_ARM_OVER_CAP_BOTTOM: ArmRegion = ArmRegion::new(55, 48, 58, 52);
const LEFT_ARM_OVER_CAP_TOP: ArmRegion = ArmRegion::new(52, 48, 55, 52);

// Void region outside standard texture areas (corner above head: [0..8, 0..8])
const VOID_TOP_LEFT_CORNER: ArmRegion = ArmRegion::new(0, 0, 8, 8);

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
    // Follow a 3-stage decision pipeline (highest certainty -> heuristic indicators -> fallback):

    // Stage 1: Absolute geometry transparency (authoritative Alex signal)
    if check_unused_transparency(image, scale) {
        return Some(SkinType::Slim);
    }

    // Stage 2: Outer overlay sleeve coverage
    if let Some(inferred) = check_overlay_sleeve_evidence(image, scale) {
        return Some(inferred);
    }

    // Stage 3: Dummy placeholder fill or boundary discontinuity
    if check_dummy_fill_or_boundary(image, scale) {
        return Some(SkinType::Slim);
    }

    // Default fallback: Classic Steve 4px arm
    Some(SkinType::Classic)
}

/// Stage 1: Checks if any active arm has transparent pixels in the Alex-unused zones.
/// In standard Minecraft skins, Classic Steve skins are completely opaque across all 4px.
fn check_unused_transparency(image: &DynamicImage, scale: u32) -> bool {
    let (r_side_act, _) = sample_region_opaque_count(image, scale, &RIGHT_ARM_BASE_ACTIVE_SIDES);
    let (r_side_un_op, r_side_un_tot) =
        sample_region_opaque_count(image, scale, &RIGHT_ARM_BASE_UNUSED_SIDES);
    let (l_side_act, _) = sample_region_opaque_count(image, scale, &LEFT_ARM_BASE_ACTIVE_SIDES);
    let (l_side_un_op, l_side_un_tot) =
        sample_region_opaque_count(image, scale, &LEFT_ARM_BASE_UNUSED_SIDES);

    let (r_cap_act, _) = sample_region_opaque_count(image, scale, &RIGHT_ARM_BASE_ACTIVE_CAP);
    let (r_cap_un_op, r_cap_un_tot) =
        sample_region_opaque_count(image, scale, &RIGHT_ARM_BASE_UNUSED_CAP);
    let (l_cap_act, _) = sample_region_opaque_count(image, scale, &LEFT_ARM_BASE_ACTIVE_CAP);
    let (l_cap_un_op, l_cap_un_tot) =
        sample_region_opaque_count(image, scale, &LEFT_ARM_BASE_UNUSED_CAP);

    let r_side_un_trans = r_side_un_tot.saturating_sub(r_side_un_op);
    let l_side_un_trans = l_side_un_tot.saturating_sub(l_side_un_op);
    let r_cap_un_trans = r_cap_un_tot.saturating_sub(r_cap_un_op);
    let l_cap_un_trans = l_cap_un_tot.saturating_sub(l_cap_un_op);

    // If an arm is drawn, and at least half the unused texels are transparent -> Slim Alex
    let r_side_slim = r_side_act > 0 && r_side_un_trans >= (r_side_un_tot / 2);
    let l_side_slim = l_side_act > 0 && l_side_un_trans >= (l_side_un_tot / 2);
    let r_cap_slim = r_cap_act > 0 && r_cap_un_trans >= (r_cap_un_tot / 2);
    let l_cap_slim = l_cap_act > 0 && l_cap_un_trans >= (l_cap_un_tot / 2);

    r_side_slim || l_side_slim || r_cap_slim || l_cap_slim
}

/// Checks if the entire texture canvas has been flood-filled with a single uniform background color
/// (e.g. solid #525252 as in `777053.png`), meaning unused columns will naturally carry this background color.
fn get_uniform_canvas_fill_color(image: &DynamicImage, scale: u32) -> Option<(u8, u8, u8)> {
    let (corner_opaque, total) = sample_region_opaque_count(image, scale, &VOID_TOP_LEFT_CORNER);
    if corner_opaque == total && total > 0 {
        let (distinct, mut colors) =
            collect_region_colors(image, scale, &VOID_TOP_LEFT_CORNER);
        if distinct == 1 {
            return colors.pop();
        }
    }
    None
}

/// Stage 2: Analyzes outer overlay sleeve layers when the base layer is completely opaque.
fn check_overlay_sleeve_evidence(image: &DynamicImage, scale: u32) -> Option<SkinType> {
    let (r_over_act, _) = sample_region_opaque_count(image, scale, &RIGHT_ARM_OVER_ACTIVE_SIDES);
    let (r_over_un, _) = sample_region_opaque_count(image, scale, &RIGHT_ARM_OVER_UNUSED_SIDES);
    let (l_over_act, _) = sample_region_opaque_count(image, scale, &LEFT_ARM_OVER_ACTIVE_SIDES);
    let (l_over_un, _) = sample_region_opaque_count(image, scale, &LEFT_ARM_OVER_UNUSED_SIDES);

    let canvas_fill = get_uniform_canvas_fill_color(image, scale);

    // If unused overlay columns are uniformly filled with the canvas background color,
    // they represent background template canvas fill rather than intentional 4px arm texturing.
    let r_un_is_bg = match canvas_fill {
        Some(bg) => {
            let (count, colors) =
                collect_region_colors(image, scale, &RIGHT_ARM_OVER_UNUSED_SIDES);
            count == 1 && colors.first() == Some(&bg)
        }
        None => false,
    };
    let l_un_is_bg = match canvas_fill {
        Some(bg) => {
            let (count, colors) =
                collect_region_colors(image, scale, &LEFT_ARM_OVER_UNUSED_SIDES);
            count == 1 && colors.first() == Some(&bg)
        }
        None => false,
    };

    // 2A: Check for 3px Alex overlay cap signatures (e.g. 3px epaulets/cuffs with 3px transparent complement).
    // In Alex UV, top cap is [44..47, 32..36] (3x4) and bottom cap is [47..50, 32..36] (3x4).
    // In Classic UV, caps are 4x4: top [44..48] and bottom [48..52].
    // An active feature drawn specifically on [47..50] (bottom cap) while [44..47] is transparent
    // is a definitive 3px Alex geometry indicator.
    if has_slim_overlay_cap_feature(image, scale, canvas_fill) {
        return Some(SkinType::Slim);
    }

    // 2B: Heavy drawing on the 4th column of overlay sleeves (>= 8 texels / 33% of 24)
    // demonstrates intentional 4px arm texturing -> Classic Steve.
    // (Only applies when the column is not just canvas background flood-fill).
    let r_heavy_classic = r_over_un >= 8 && !r_un_is_bg;
    let l_heavy_classic = l_over_un >= 8 && !l_un_is_bg;
    if r_heavy_classic || l_heavy_classic {
        return Some(SkinType::Classic);
    }

    // 2C: Clean tailored 3px overlay sleeves: substantial sleeve texture present (>= 20 texels),
    // but the 4th column strip is left untouched (transparent or background fill) -> Slim Alex.
    let r_clean_slim = r_over_act >= 20 && (r_over_un <= 1 || r_un_is_bg);
    let l_clean_slim = l_over_act >= 20 && (l_over_un <= 1 || l_un_is_bg);
    if r_clean_slim || l_clean_slim {
        return Some(SkinType::Slim);
    }

    None
}

/// Detects distinct 3px Alex overlay cap features (e.g. golden epaulets or shoulder pads).
///
/// In Minecraft UV:
/// - Alex bottom cap is [47..50, 32..36] (Right) and [55..58, 48..52] (Left), exactly 3x4 texels.
/// - Classic Steve caps are 4x4: Top is [44..48] / [52..56], and Bottom is [48..52] / [56..60].
///
/// An active 3px feature drawn across [47..50] (or [55..58]) while the top cap [44..47] (or [52..55])
/// is completely empty straddles the Steve 4px cap boundary (x=47|48 or 55|56). This cannot occur
/// in standard Classic texturing and is a definitive 3px Alex bottom cap indicator.
fn has_slim_overlay_cap_feature(
    image: &DynamicImage,
    scale: u32,
    canvas_fill: Option<(u8, u8, u8)>,
) -> bool {
    let check_arm = |top: &ArmRegion, bot: &ArmRegion| -> bool {
        let (top_op, _) = sample_region_opaque_count(image, scale, top);
        let (bot_op, bot_tot) = sample_region_opaque_count(image, scale, bot);

        let (bot_non_bg, bot_distinct) = count_region_non_bg_colors(image, scale, bot, canvas_fill);

        // Pattern: Bottom cap has active artwork (e.g. epaulet) while top cap is empty/transparent.
        // In Steve, top cap is [44..48], so [47..50] crossing into [48] cannot happen as a single 3px block.
        top_op == 0 && bot_op == bot_tot && bot_non_bg >= (bot_tot / 2) && bot_distinct >= 2
    };

    check_arm(&RIGHT_ARM_OVER_CAP_TOP, &RIGHT_ARM_OVER_CAP_BOTTOM)
        || check_arm(&LEFT_ARM_OVER_CAP_TOP, &LEFT_ARM_OVER_CAP_BOTTOM)
}

fn count_region_non_bg_colors(
    image: &DynamicImage,
    scale: u32,
    region: &ArmRegion,
    canvas_fill: Option<(u8, u8, u8)>,
) -> (u32, usize) {
    let (w, h) = image.dimensions();
    let mut non_bg_count = 0;
    let mut colors = Vec::new();

    for by in region.by0..region.by1 {
        for bx in region.bx0..region.bx1 {
            let px = ((bx as u64 * 2 + 1) * scale as u64 / 2).min((w - 1) as u64) as u32;
            let py = ((by as u64 * 2 + 1) * scale as u64 / 2).min((h - 1) as u64) as u32;
            let p = image.get_pixel(px, py);
            if p[3] > 10 {
                let rgb = (p[0], p[1], p[2]);
                if Some(rgb) != canvas_fill {
                    non_bg_count += 1;
                    if !colors.contains(&rgb) {
                        colors.push(rgb);
                    }
                }
            }
        }
    }

    (non_bg_count, colors.len())
}

/// Stage 3: Checks for dummy solid color fills (e.g. flat #000000 or skin tone) or severe boundary discontinuities.
fn check_dummy_fill_or_boundary(image: &DynamicImage, scale: u32) -> bool {
    let r_base_colors = count_region_distinct_colors(image, scale, &RIGHT_ARM_BASE_UNUSED_SIDES);
    let l_base_colors = count_region_distinct_colors(image, scale, &LEFT_ARM_BASE_UNUSED_SIDES);
    let r_cap_colors = count_region_distinct_colors(image, scale, &RIGHT_ARM_BASE_UNUSED_CAP);
    let l_cap_colors = count_region_distinct_colors(image, scale, &LEFT_ARM_BASE_UNUSED_CAP);

    // Discontinuity across column 53 (arm edge) and 54 (unused strip start) on right arm,
    // and across column 45 (arm edge) and 46 (unused strip start) on left arm.
    let diff_r = avg_column_color_diff(image, scale, 53, 54, 20, 32);
    let diff_l = avg_column_color_diff(image, scale, 45, 46, 52, 64);

    // 3A: Placeholder dummy fill (e.g. solid #000000 with sharp color boundary jump).
    if ((r_base_colors <= 2 && diff_r > 70.0) || (l_base_colors <= 2 && diff_l > 70.0))
        && (r_cap_colors <= 2 || l_cap_colors <= 2)
    {
        return true;
    }

    // 3B: Dual-arm boundary discontinuity with completely untouched overlay.
    let (r_over_un, _) = sample_region_opaque_count(image, scale, &RIGHT_ARM_OVER_UNUSED_SIDES);
    let (l_over_un, _) = sample_region_opaque_count(image, scale, &LEFT_ARM_OVER_UNUSED_SIDES);
    if diff_r > 150.0 && diff_l > 150.0 && r_over_un == 0 && l_over_un == 0 {
        return true;
    }

    false
}

/// Infer skin arm model from raw image bytes.
pub fn infer_skin_type_from_bytes(bytes: &[u8]) -> Result<Option<SkinType>, EidolonError> {
    let img = image::load_from_memory(bytes)?;
    Ok(infer_skin_type(&img))
}

fn sample_region_opaque_count(
    image: &DynamicImage,
    scale: u32,
    region: &ArmRegion,
) -> (u32, u32) {
    sample_grid_opaque_count(image, scale, region.bx0, region.by0, region.bx1, region.by1)
}

fn collect_region_colors(
    image: &DynamicImage,
    scale: u32,
    region: &ArmRegion,
) -> (usize, Vec<(u8, u8, u8)>) {
    let (w, h) = image.dimensions();
    let mut colors: Vec<(u8, u8, u8)> = Vec::new();

    for by in region.by0..region.by1 {
        for bx in region.bx0..region.bx1 {
            let px = ((bx as u64 * 2 + 1) * scale as u64 / 2).min((w - 1) as u64) as u32;
            let py = ((by as u64 * 2 + 1) * scale as u64 / 2).min((h - 1) as u64) as u32;
            let p = image.get_pixel(px, py);
            if p[3] > 10 {
                let rgb = (p[0], p[1], p[2]);
                if !colors.contains(&rgb) {
                    colors.push(rgb);
                }
            }
        }
    }

    (colors.len(), colors)
}

fn count_region_distinct_colors(
    image: &DynamicImage,
    scale: u32,
    region: &ArmRegion,
) -> usize {
    collect_region_colors(image, scale, region).0
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
            (
                "identify/121251.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/132028.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/185975.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Slim),
            ),
            (
                "identify/185978.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Slim),
            ),
            (
                "identify/271991.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Slim),
            ),
            (
                "identify/335300.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/345827.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/372012.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/777053.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Slim),
            ),
            (
                "identify/10605.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/105214.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/108588.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/113152.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
            ),
            (
                "identify/185977.png",
                TextureKind::SkinDoubleLayer,
                Some(SkinType::Classic),
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
