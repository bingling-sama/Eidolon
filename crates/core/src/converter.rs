//! Minecraft skin single-to-double layer layout conversion.
//!
//! Re-exports single-to-double layer transformation functions from `assets`.

use crate::error::EidolonError;
use image::DynamicImage;

/// Expand a legacy single-layer skin (`width == 2 * height`) to a square double-layer atlas by
/// copying the top half and synthesizing mirrored left limbs in the bottom half.
pub fn single2double(img: &DynamicImage) -> Result<DynamicImage, EidolonError> {
    crate::assets::single2double(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single2double_reexport_delegation() {
        let img = image::open("resources/SSSSSteven.png")
            .expect("SSSSSteven.png not found — run from project root");
        let result = single2double(&img);
        assert!(result.is_ok(), "conversion failed: {:?}", result.err());
        let out = result.unwrap();
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 64);
    }
}
