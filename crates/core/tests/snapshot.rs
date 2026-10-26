//! Snapshot regression test for the render pipeline.
//!
//! Renders a known character+camera config and compares the pixel hash against
//! a golden constant. Catches silent rendering changes (shader, matrix, model).
//!
//! To update the golden hash after an intentional rendering change:
//! run this test, copy the actual hash from the panic message, paste into
//! the GOLDEN_HASH constant below.

#![cfg(not(target_arch = "wasm32"))]

mod common;

use common::resource_str;
use eidolon_core::camera::Camera;
use eidolon_core::character::{Character, Posture, SkinType};
use eidolon_core::renderer::Renderer;

/// Golden pixel hashes, one entry per rendering environment. Rasterizer output differs
/// between backends, so a new environment (or an intentional rendering change) only needs its
/// own value: run the test and copy the hash it prints.
///
/// - 0xed3ec08a32ce058d: Windows / Vulkan
/// - 0x80fec08a32d1d01c: Linux / Mesa (WSL2)
///
/// The GitHub Actions runner uses a different Mesa build, so it still needs its own value:
/// run the test there and copy the hash it prints.
const ACCEPTED_GOLDEN_HASHES: &[u64] = &[
    17095312935335363981, // 0xed3ec08a32ce058d
    9295078380729913372,  // 0x80fec08a32d1d01c
];

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
fn snapshot_classic_standing_front() {
    let renderer = Renderer::new().expect("Failed to create Renderer");

    let body = renderer
        .load_body(&resource_str("bingling_sama.png"), SkinType::Classic)
        .expect("Failed to load body");
    let mut character = Character::from_body(body);
    character.posture = Posture {
        cape_pitch: 10.8,
        body_pitch: 0.0,
        elytra_pitch: 15.0,
        elytra_yaw: 0.5729578,
        elytra_roll: 15.0,
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 90.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 90.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
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

    if ACCEPTED_GOLDEN_HASHES.is_empty() {
        panic!(
            "ACCEPTED_GOLDEN_HASHES empty. Update tests/snapshot.rs with:\n{}",
            hash
        );
    }

    assert!(
        ACCEPTED_GOLDEN_HASHES.contains(&hash),
        "Snapshot hash changed!\n  actual:   0x{:016x} ({})\n  accepted: {:x?}",
        hash,
        hash,
        ACCEPTED_GOLDEN_HASHES
            .iter()
            .map(|h| format!("0x{:016x}", h))
            .collect::<Vec<_>>()
    );
}
