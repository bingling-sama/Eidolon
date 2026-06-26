//! Integration tests for the eidolon renderer.
//!
//! Exercise the full public API: Character → Renderer → ImageBuffer → file.
//! All tests are `#[ignore]` — they require a GPU or software rasterizer.
//!
//! Run with: `cargo test -- --ignored`

use eidolon::camera::Camera;
use eidolon::character::{Character, Posture, SkinType};
use eidolon::renderer::{OutputFormat, Renderer};
use eidolon::texture::Texture;

fn make_renderer() -> Renderer {
    Renderer::new().expect("Failed to create Renderer")
}

fn character_with_skin(renderer: &Renderer) -> (Character, Texture) {
    let mut c = Character::new();
    c.skin_type = SkinType::Classic;
    c.posture = Posture {
        head_yaw: 0.0,
        head_pitch: 0.0,
        left_arm_roll: 90.0,
        left_arm_pitch: 0.0,
        right_arm_roll: 90.0,
        right_arm_pitch: 0.0,
        left_leg_pitch: 0.0,
        right_leg_pitch: 0.0,
    };
    let skin = renderer
        .load_texture("resources/bingling_sama.png")
        .expect("Failed to load skin");
    (c, skin)
}

fn camera_default() -> Camera {
    Camera {
        yaw: 180.0,
        pitch: 90.0,
        scale: 1.0,
    }
}

#[test]
#[ignore]
fn render_produces_correct_dimensions() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);
    let image = renderer
        .render(&character, &skin, &camera_default(), 800, 600)
        .expect("Render failed");
    assert_eq!(image.width(), 800);
    assert_eq!(image.height(), 600);
}

#[test]
#[ignore]
fn render_output_not_blank() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);
    let image = renderer
        .render(&character, &skin, &camera_default(), 800, 600)
        .expect("Render failed");

    let non_bg: usize = image
        .pixels()
        .filter(|p| p[0] > 60 || p[1] > 60 || p[2] > 110)
        .count();
    assert!(
        non_bg > 100,
        "Only {} non-background pixels — render appears blank",
        non_bg
    );
}

#[test]
#[ignore]
fn different_camera_angle_produces_different_output() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);

    let front = renderer
        .render(&character, &skin, &Camera { yaw: 90.0, pitch: 90.0, scale: 1.0 }, 200, 150)
        .expect("Front render failed");
    let back = renderer
        .render(&character, &skin, &Camera { yaw: 270.0, pitch: 90.0, scale: 1.0 }, 200, 150)
        .expect("Back render failed");

    let px_front: Vec<u8> = front.pixels().flat_map(|p| p.0.to_vec()).collect();
    let px_back: Vec<u8> = back.pixels().flat_map(|p| p.0.to_vec()).collect();
    assert_ne!(px_front, px_back, "Front and back renders must differ");
}

#[test]
#[ignore]
fn render_to_image_saves_to_disk() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);

    let tmp = std::env::temp_dir().join("eidolon_test_output.png");
    let tmp_str = tmp.to_str().expect("temp path not UTF-8");
    renderer
        .render_to_image(&character, &skin, &camera_default(), tmp_str, (200, 150), OutputFormat::Png)
        .expect("render_to_image failed");

    assert!(tmp.exists(), "Output file not created: {:?}", tmp);
    assert!(std::fs::metadata(&tmp).unwrap().len() > 0, "Output file empty");
    std::fs::remove_file(&tmp).ok();
}

#[test]
#[ignore]
fn webp_output_format_works() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);

    let tmp = std::env::temp_dir().join("eidolon_test_output.webp");
    let tmp_str = tmp.to_str().expect("temp path not UTF-8");
    renderer
        .render_to_image(&character, &skin, &camera_default(), tmp_str, (200, 150), OutputFormat::WebP)
        .expect("render_to_image (webp) failed");

    assert!(tmp.exists(), "WebP output file not created");
    assert!(std::fs::metadata(&tmp).unwrap().len() > 0, "WebP output empty");
    std::fs::remove_file(&tmp).ok();
}

#[test]
#[ignore]
fn slim_skin_type_renders() {
    let renderer = make_renderer();
    let (mut character, skin) = character_with_skin(&renderer);
    character.skin_type = SkinType::Slim;
    let image = renderer
        .render(&character, &skin, &camera_default(), 400, 300)
        .expect("Slim render failed");
    assert_eq!(image.width(), 400);
    assert_eq!(image.height(), 300);
}

#[test]
#[ignore]
fn load_texture_invalid_path_returns_error() {
    let renderer = make_renderer();
    let result = renderer.load_texture("nonexistent_skin_xyz.png");
    match result {
        Ok(_) => panic!("Expected error for nonexistent path"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("failed to resolve"),
                "Expected path resolution error, got: {}",
                msg
            );
        }
    }
}
