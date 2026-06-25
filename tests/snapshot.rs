//! Snapshot regression test for the render pipeline.
//!
//! Renders a known character+camera config and compares the pixel hash against
//! a golden constant. Catches silent rendering changes (shader, matrix, model).
//!
//! Run with: `cargo test -- --ignored`
//!
//! To update the golden hash after an intentional rendering change:
//! run this test, copy the actual hash from the panic message, paste into
//! the GOLDEN_HASH constant below.

use eidolon::camera::Camera;
use eidolon::character::{Character, Posture, SkinType};
use eidolon::renderer::Renderer;

/// Golden pixel hash — update when rendering changes intentionally.
///
/// Config: classic skin, bingling_sama.png, yaw=180 pitch=90 scale=1,
/// standing posture (all defaults), 800×600.
const GOLDEN_HASH: u64 = 0; // placeholder — first wgpu run sets this

fn hash_pixels(width: u32, height: u32, pixels: &[u8]) -> u64 {
    let mut h: u64 = 0;
    let pixel_count = (width * height) as usize;
    for (i, chunk) in pixels.chunks(4).take(pixel_count).enumerate() {
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        h ^= (word as u64).rotate_left((i % 64) as u32);
    }
    h
}

#[test]
#[ignore]
fn snapshot_classic_standing_front() {
    let renderer = Renderer::new().expect("Failed to create Renderer");

    let mut character = Character::new();
    character.skin_type = SkinType::Classic;
    character.posture = Posture {
        head_yaw: 90.0,
        head_pitch: 90.0,
        left_arm_roll: 90.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 90.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 90.0,
        right_leg_pitch: 90.0,
    };
    character.skin = Some(
        renderer
            .load_texture("resources/bingling_sama.png")
            .expect("Failed to load skin"),
    );

    let camera = Camera {
        yaw: 180.0,
        pitch: 90.0,
        scale: 1.0,
    };

    let image = renderer
        .render(&character, &camera, 800, 600)
        .expect("Snapshot render failed");

    let raw: Vec<u8> = image
        .pixels()
        .flat_map(|p| [p[0], p[1], p[2], p[3]])
        .collect();
    let hash = hash_pixels(800, 600, &raw);

    if GOLDEN_HASH == 0 {
        panic!(
            "GOLDEN_HASH not set. Update tests/snapshot.rs:\nconst GOLDEN_HASH: u64 = {};",
            hash
        );
    }

    assert_eq!(
        hash, GOLDEN_HASH,
        "Snapshot hash changed!\n  expected: 0x{:016x}\n  actual:   0x{:016x}",
        GOLDEN_HASH, hash
    );
}
