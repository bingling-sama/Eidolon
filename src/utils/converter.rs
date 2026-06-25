use image::{imageops, DynamicImage, GenericImageView, ImageBuffer};

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
pub fn single2double(img: &DynamicImage) -> Result<DynamicImage, String> {
    // Check if the image is single-layer (width is twice the height)
    if img.width() != img.height() * 2 {
        return Err(
            "Input image is not a single-layer skin (width must be twice the height).".to_string(),
        );
    }

    let hd_ratio = img.width() as f32 / 64.0;
    let scale = |rect: (u32, u32, u32, u32)| scale_rect(rect, hd_ratio);

    // Create a new square image buffer for the double-layer skin
    let mut output_img = ImageBuffer::new(img.width(), img.width());

    // Copy the original single-layer skin to the top half of the new image
    for y in 0..img.height() {
        for x in 0..img.width() {
            let pixel = img.get_pixel(x, y);
            output_img.put_pixel(x, y, pixel);
        }
    }

    // Map each right-side source region (top half) to its left-side
    // destination (bottom half): crop → flip horizontally → overlay
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba};

    #[test]
    fn test_single2double_success() {
        let img = image::open("resources/SSSSSteven.png")
            .expect("SSSSSteven.png not found — run from project root");
        let result = single2double(&img);
        assert!(result.is_ok(), "conversion failed: {:?}", result.err());
        let out = result.unwrap();
        // Output atlas is 64×64 for a 64×32 input.
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 64);
        // Top half must match the original strip.
        for y in 0..32 {
            for x in 0..64 {
                assert_eq!(out.get_pixel(x, y), img.get_pixel(x, y));
            }
        }
    }

    #[test]
    #[test]
    fn test_single2double_success_synthetic() {
        // 64x32 synthetic single-layer skin
        let img =
            DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(64, 32, Rgba([255, 0, 0, 255])));
        let result = single2double(&img);
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 64);
    }

    #[test]
    fn test_single2double_invalid_aspect_ratio() {
        // width != 2 * height — should error
        let img =
            DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(64, 31, Rgba([0, 0, 0, 255])));
        assert!(single2double(&img).is_err());

        // square image is also invalid
        let img2 =
            DynamicImage::ImageRgba8(image::ImageBuffer::from_pixel(64, 64, Rgba([0, 0, 0, 255])));
        assert!(single2double(&img2).is_err());
    }

    #[test]
    fn test_scale_rect_identity() {
        // hd_ratio=1.0 → no scaling
        let rect = (10, 20, 30, 40);
        let scaled = scale_rect(rect, 1.0);
        assert_eq!(scaled, rect);
    }

    #[test]
    fn test_scale_rect_hd_doubling() {
        // hd_ratio=2.0 for 128px wide skin
        let scaled = scale_rect((10, 20, 30, 40), 2.0);
        assert_eq!(scaled, (20, 40, 60, 80));
    }
}
