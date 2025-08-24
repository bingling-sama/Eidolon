use image::{imageops, GenericImageView, ImageBuffer, DynamicImage};
use std::path::Path;

// 64px 右腿 OFIB (Outside, Front, Inside, Back) - source regions
const RIGHT_LEG_OUTSIDE_RANGE: (u32, u32, u32, u32) = (0, 20, 4, 32);
const RIGHT_LEG_TOP_FRONT_RANGE: (u32, u32, u32, u32) = (4, 16, 8, 32);
const RIGHT_LEG_BUTTOM_RANGE: (u32, u32, u32, u32) = (8, 16, 12, 20);
const RIGHT_LEG_INSIDE_RANGE: (u32, u32, u32, u32) = (8, 20, 12, 32);
const RIGHT_LEG_BACK_RANGE: (u32, u32, u32, u32) = (12, 20, 16, 32);

// 64px 左腿 IFOB (Inside, Front, Outside, Back) - destination regions
const LEFT_LEG_OUTSIDE_RANGE: (u32, u32, u32, u32) = (24, 52, 28, 64);
const LEFT_LEG_TOP_FRONT_RANGE: (u32, u32, u32, u32) = (20, 48, 24, 64);
const LEFT_LEG_BUTTOM_RANGE: (u32, u32, u32, u32) = (24, 48, 28, 52);
const LEFT_LEG_INSIDE_RANGE: (u32, u32, u32, u32) = (16, 52, 20, 64);
const LEFT_LEG_BACK_RANGE: (u32, u32, u32, u32) = (28, 52, 32, 64);

// 64px 右臂 OFIB - source regions
const RIGHT_ARM_OUTSIDE_RANGE: (u32, u32, u32, u32) = (40, 20, 44, 32);
const RIGHT_ARM_TOP_RANGE: (u32, u32, u32, u32) = (44, 16, 48, 20);
const RIGHT_ARM_FRONT_RANGE: (u32, u32, u32, u32) = (44, 20, 48, 32);
const RIGHT_ARM_BUTTOM_RANGE: (u32, u32, u32, u32) = (48, 16, 52, 20);
const RIGHT_ARM_INSIDE_RANGE: (u32, u32, u32, u32) = (48, 20, 52, 32);
const RIGHT_ARM_BACK_RANGE: (u32, u32, u32, u32) = (52, 20, 56, 32);

// 64px 左臂 IFOB - destination regions
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
/// 将单层皮肤的DynamicImage转换为双层DynamicImage
pub fn single2double_image(img: &DynamicImage) -> Result<DynamicImage, String> {
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

    // Scale all regions
    let right_leg_outside_area = scale(RIGHT_LEG_OUTSIDE_RANGE);
    let right_leg_top_front_area = scale(RIGHT_LEG_TOP_FRONT_RANGE);
    let right_leg_buttom_area = scale(RIGHT_LEG_BUTTOM_RANGE);
    let right_leg_inside_area = scale(RIGHT_LEG_INSIDE_RANGE);
    let right_leg_back_area = scale(RIGHT_LEG_BACK_RANGE);

    let right_arm_outside_area = scale(RIGHT_ARM_OUTSIDE_RANGE);
    let right_arm_top_area = scale(RIGHT_ARM_TOP_RANGE);
    let right_arm_front_area = scale(RIGHT_ARM_FRONT_RANGE);
    let right_arm_buttom_area = scale(RIGHT_ARM_BUTTOM_RANGE);
    let right_arm_inside_area = scale(RIGHT_ARM_INSIDE_RANGE);
    let right_arm_back_area = scale(RIGHT_ARM_BACK_RANGE);

    let left_leg_outside_area = scale(LEFT_LEG_OUTSIDE_RANGE);
    let left_leg_top_front_area = scale(LEFT_LEG_TOP_FRONT_RANGE);
    let left_leg_buttom_area = scale(LEFT_LEG_BUTTOM_RANGE);
    let left_leg_inside_area = scale(LEFT_LEG_INSIDE_RANGE);
    let left_leg_back_area = scale(LEFT_LEG_BACK_RANGE);

    let left_arm_outside_area = scale(LEFT_ARM_OUTSIDE_RANGE);
    let left_arm_top_area = scale(LEFT_ARM_TOP_RANGE);
    let left_arm_front_area = scale(LEFT_ARM_FRONT_RANGE);
    let left_arm_buttom_area = scale(LEFT_ARM_BUTTOM_RANGE);
    let left_arm_inside_area = scale(LEFT_ARM_INSIDE_RANGE);
    let left_arm_back_area = scale(LEFT_ARM_BACK_RANGE);

    // Crop right leg parts
    let right_leg_top_front = img
        .view(
            right_leg_top_front_area.0,
            right_leg_top_front_area.1,
            right_leg_top_front_area.2 - right_leg_top_front_area.0,
            right_leg_top_front_area.3 - right_leg_top_front_area.1,
        )
        .to_image();
    let right_leg_buttom = img
        .view(
            right_leg_buttom_area.0,
            right_leg_buttom_area.1,
            right_leg_buttom_area.2 - right_leg_buttom_area.0,
            right_leg_buttom_area.3 - right_leg_buttom_area.1,
        )
        .to_image();
    let right_leg_inside = img
        .view(
            right_leg_inside_area.0,
            right_leg_inside_area.1,
            right_leg_inside_area.2 - right_leg_inside_area.0,
            right_leg_inside_area.3 - right_leg_inside_area.1,
        )
        .to_image();
    let right_leg_back = img
        .view(
            right_leg_back_area.0,
            right_leg_back_area.1,
            right_leg_back_area.2 - right_leg_back_area.0,
            right_leg_back_area.3 - right_leg_back_area.1,
        )
        .to_image();
    let right_leg_outside = img
        .view(
            right_leg_outside_area.0,
            right_leg_outside_area.1,
            right_leg_outside_area.2 - right_leg_outside_area.0,
            right_leg_outside_area.3 - right_leg_outside_area.1,
        )
        .to_image();

    // Crop right arm parts
    let right_arm_top = img
        .view(
            right_arm_top_area.0,
            right_arm_top_area.1,
            right_arm_top_area.2 - right_arm_top_area.0,
            right_arm_top_area.3 - right_arm_top_area.1,
        )
        .to_image();
    let right_arm_front = img
        .view(
            right_arm_front_area.0,
            right_arm_front_area.1,
            right_arm_front_area.2 - right_arm_front_area.0,
            right_arm_front_area.3 - right_arm_front_area.1,
        )
        .to_image();
    let right_arm_buttom = img
        .view(
            right_arm_buttom_area.0,
            right_arm_buttom_area.1,
            right_arm_buttom_area.2 - right_arm_buttom_area.0,
            right_arm_buttom_area.3 - right_arm_buttom_area.1,
        )
        .to_image();
    let right_arm_inside = img
        .view(
            right_arm_inside_area.0,
            right_arm_inside_area.1,
            right_arm_inside_area.2 - right_arm_inside_area.0,
            right_arm_inside_area.3 - right_arm_inside_area.1,
        )
        .to_image();
    let right_arm_back = img
        .view(
            right_arm_back_area.0,
            right_arm_back_area.1,
            right_arm_back_area.2 - right_arm_back_area.0,
            right_arm_back_area.3 - right_arm_back_area.1,
        )
        .to_image();
    let right_arm_outside = img
        .view(
            right_arm_outside_area.0,
            right_arm_outside_area.1,
            right_arm_outside_area.2 - right_arm_outside_area.0,
            right_arm_outside_area.3 - right_arm_outside_area.1,
        )
        .to_image();

    // Flip right arm to create left arm (horizontal flip)
    let left_arm_outside = imageops::flip_horizontal(&right_arm_outside);
    let left_arm_top = imageops::flip_horizontal(&right_arm_top);
    let left_arm_front = imageops::flip_horizontal(&right_arm_front);
    let left_arm_buttom = imageops::flip_horizontal(&right_arm_buttom);
    let left_arm_inside = imageops::flip_horizontal(&right_arm_inside);
    let left_arm_back = imageops::flip_horizontal(&right_arm_back);

    // Flip right leg to create left leg (horizontal flip)
    let left_leg_outside = imageops::flip_horizontal(&right_leg_outside);
    let left_leg_top_front = imageops::flip_horizontal(&right_leg_top_front);
    let left_leg_buttom = imageops::flip_horizontal(&right_leg_buttom);
    let left_leg_inside = imageops::flip_horizontal(&right_leg_inside);
    let left_leg_back = imageops::flip_horizontal(&right_leg_back);

    // Paste left arm parts
    imageops::overlay(
        &mut output_img,
        &left_arm_outside,
        left_arm_outside_area.0 as i64,
        left_arm_outside_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_top,
        left_arm_top_area.0 as i64,
        left_arm_top_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_front,
        left_arm_front_area.0 as i64,
        left_arm_front_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_buttom,
        left_arm_buttom_area.0 as i64,
        left_arm_buttom_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_inside,
        left_arm_inside_area.0 as i64,
        left_arm_inside_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_back,
        left_arm_back_area.0 as i64,
        left_arm_back_area.1 as i64,
    );

    // Paste left leg parts
    imageops::overlay(
        &mut output_img,
        &left_leg_top_front,
        left_leg_top_front_area.0 as i64,
        left_leg_top_front_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_buttom,
        left_leg_buttom_area.0 as i64,
        left_leg_buttom_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_inside,
        left_leg_inside_area.0 as i64,
        left_leg_inside_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_back,
        left_leg_back_area.0 as i64,
        left_leg_back_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_outside,
        left_leg_outside_area.0 as i64,
        left_leg_outside_area.1 as i64,
    );

    Ok(DynamicImage::ImageRgba8(output_img))
}

pub fn single2double(input_path: &Path, output_path: &Path) -> Result<(), String> {
    // Load the input image
    let img = image::open(input_path).map_err(|e| format!("Failed to open input image: {}", e))?;

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

    // Scale all regions
    let right_leg_outside_area = scale(RIGHT_LEG_OUTSIDE_RANGE);
    let right_leg_top_front_area = scale(RIGHT_LEG_TOP_FRONT_RANGE);
    let right_leg_buttom_area = scale(RIGHT_LEG_BUTTOM_RANGE);
    let right_leg_inside_area = scale(RIGHT_LEG_INSIDE_RANGE);
    let right_leg_back_area = scale(RIGHT_LEG_BACK_RANGE);

    let right_arm_outside_area = scale(RIGHT_ARM_OUTSIDE_RANGE);
    let right_arm_top_area = scale(RIGHT_ARM_TOP_RANGE);
    let right_arm_front_area = scale(RIGHT_ARM_FRONT_RANGE);
    let right_arm_buttom_area = scale(RIGHT_ARM_BUTTOM_RANGE);
    let right_arm_inside_area = scale(RIGHT_ARM_INSIDE_RANGE);
    let right_arm_back_area = scale(RIGHT_ARM_BACK_RANGE);

    let left_leg_outside_area = scale(LEFT_LEG_OUTSIDE_RANGE);
    let left_leg_top_front_area = scale(LEFT_LEG_TOP_FRONT_RANGE);
    let left_leg_buttom_area = scale(LEFT_LEG_BUTTOM_RANGE);
    let left_leg_inside_area = scale(LEFT_LEG_INSIDE_RANGE);
    let left_leg_back_area = scale(LEFT_LEG_BACK_RANGE);

    let left_arm_outside_area = scale(LEFT_ARM_OUTSIDE_RANGE);
    let left_arm_top_area = scale(LEFT_ARM_TOP_RANGE);
    let left_arm_front_area = scale(LEFT_ARM_FRONT_RANGE);
    let left_arm_buttom_area = scale(LEFT_ARM_BUTTOM_RANGE);
    let left_arm_inside_area = scale(LEFT_ARM_INSIDE_RANGE);
    let left_arm_back_area = scale(LEFT_ARM_BACK_RANGE);

    // Crop right leg parts
    let right_leg_top_front = img
        .view(
            right_leg_top_front_area.0,
            right_leg_top_front_area.1,
            right_leg_top_front_area.2 - right_leg_top_front_area.0,
            right_leg_top_front_area.3 - right_leg_top_front_area.1,
        )
        .to_image();
    let right_leg_buttom = img
        .view(
            right_leg_buttom_area.0,
            right_leg_buttom_area.1,
            right_leg_buttom_area.2 - right_leg_buttom_area.0,
            right_leg_buttom_area.3 - right_leg_buttom_area.1,
        )
        .to_image();
    let right_leg_inside = img
        .view(
            right_leg_inside_area.0,
            right_leg_inside_area.1,
            right_leg_inside_area.2 - right_leg_inside_area.0,
            right_leg_inside_area.3 - right_leg_inside_area.1,
        )
        .to_image();
    let right_leg_back = img
        .view(
            right_leg_back_area.0,
            right_leg_back_area.1,
            right_leg_back_area.2 - right_leg_back_area.0,
            right_leg_back_area.3 - right_leg_back_area.1,
        )
        .to_image();
    let right_leg_outside = img
        .view(
            right_leg_outside_area.0,
            right_leg_outside_area.1,
            right_leg_outside_area.2 - right_leg_outside_area.0,
            right_leg_outside_area.3 - right_leg_outside_area.1,
        )
        .to_image();

    // Crop right arm parts
    let right_arm_top = img
        .view(
            right_arm_top_area.0,
            right_arm_top_area.1,
            right_arm_top_area.2 - right_arm_top_area.0,
            right_arm_top_area.3 - right_arm_top_area.1,
        )
        .to_image();
    let right_arm_front = img
        .view(
            right_arm_front_area.0,
            right_arm_front_area.1,
            right_arm_front_area.2 - right_arm_front_area.0,
            right_arm_front_area.3 - right_arm_front_area.1,
        )
        .to_image();
    let right_arm_buttom = img
        .view(
            right_arm_buttom_area.0,
            right_arm_buttom_area.1,
            right_arm_buttom_area.2 - right_arm_buttom_area.0,
            right_arm_buttom_area.3 - right_arm_buttom_area.1,
        )
        .to_image();
    let right_arm_inside = img
        .view(
            right_arm_inside_area.0,
            right_arm_inside_area.1,
            right_arm_inside_area.2 - right_arm_inside_area.0,
            right_arm_inside_area.3 - right_arm_inside_area.1,
        )
        .to_image();
    let right_arm_back = img
        .view(
            right_arm_back_area.0,
            right_arm_back_area.1,
            right_arm_back_area.2 - right_arm_back_area.0,
            right_arm_back_area.3 - right_arm_back_area.1,
        )
        .to_image();
    let right_arm_outside = img
        .view(
            right_arm_outside_area.0,
            right_arm_outside_area.1,
            right_arm_outside_area.2 - right_arm_outside_area.0,
            right_arm_outside_area.3 - right_arm_outside_area.1,
        )
        .to_image();

    // Flip right arm to create left arm (horizontal flip)
    let left_arm_outside = imageops::flip_horizontal(&right_arm_outside);
    let left_arm_top = imageops::flip_horizontal(&right_arm_top);
    let left_arm_front = imageops::flip_horizontal(&right_arm_front);
    let left_arm_buttom = imageops::flip_horizontal(&right_arm_buttom);
    let left_arm_inside = imageops::flip_horizontal(&right_arm_inside);
    let left_arm_back = imageops::flip_horizontal(&right_arm_back);

    // Flip right leg to create left leg (horizontal flip)
    let left_leg_outside = imageops::flip_horizontal(&right_leg_outside);
    let left_leg_top_front = imageops::flip_horizontal(&right_leg_top_front);
    let left_leg_buttom = imageops::flip_horizontal(&right_leg_buttom);
    let left_leg_inside = imageops::flip_horizontal(&right_leg_inside);
    let left_leg_back = imageops::flip_horizontal(&right_leg_back);

    // Paste left arm parts
    imageops::overlay(
        &mut output_img,
        &left_arm_outside,
        left_arm_outside_area.0 as i64,
        left_arm_outside_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_top,
        left_arm_top_area.0 as i64,
        left_arm_top_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_front,
        left_arm_front_area.0 as i64,
        left_arm_front_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_buttom,
        left_arm_buttom_area.0 as i64,
        left_arm_buttom_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_inside,
        left_arm_inside_area.0 as i64,
        left_arm_inside_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_arm_back,
        left_arm_back_area.0 as i64,
        left_arm_back_area.1 as i64,
    );

    // Paste left leg parts
    imageops::overlay(
        &mut output_img,
        &left_leg_top_front,
        left_leg_top_front_area.0 as i64,
        left_leg_top_front_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_buttom,
        left_leg_buttom_area.0 as i64,
        left_leg_buttom_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_inside,
        left_leg_inside_area.0 as i64,
        left_leg_inside_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_back,
        left_leg_back_area.0 as i64,
        left_leg_back_area.1 as i64,
    );
    imageops::overlay(
        &mut output_img,
        &left_leg_outside,
        left_leg_outside_area.0 as i64,
        left_leg_outside_area.1 as i64,
    );

    // Save the output image
    output_img
        .save(output_path)
        .map_err(|e| format!("Failed to save output image: {}", e))?;

    Ok(())
}
