//! Integration tests for the eidolon renderer.
//!
//! Exercise the full public API: Character → Renderer → ImageBuffer → file.
//! Headless via OSMesa — no GPU required.

#![cfg(not(target_arch = "wasm32"))]

use eidolon::camera::Camera;
use eidolon::character::{Character, DefaultPostures, Posture, SkinType};
use eidolon::model::Model;
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

#[test]
fn load_model_from_bytes_matches_from_file() {
    let slim_bytes = std::fs::read("resources/slim.obj").expect("Failed to read slim.obj");
    let classic_bytes = std::fs::read("resources/classic.obj").expect("Failed to read classic.obj");

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("No wgpu adapter");
    let (device, _queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("No wgpu device");

    let from_file = Model::load_from_obj(&device, "resources/slim.obj")
        .expect("load_from_obj slim failed");
    let from_bytes =
        Model::load_from_obj_bytes(&device, &slim_bytes, "slim.obj")
            .expect("load_from_obj_bytes slim failed");

    // Both should have all parts.
    assert!(from_file.head.main.vertex_count > 0);
    assert!(from_bytes.head.main.vertex_count > 0);
    assert!(from_file.body.main.vertex_count > 0);
    assert!(from_bytes.body.main.vertex_count > 0);

    // Classic from bytes.
    Model::load_from_obj_bytes(&device, &classic_bytes, "classic.obj")
        .expect("load_from_obj_bytes classic failed");
}

#[test]
fn load_model_from_bytes_invalid_data_errors() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("No wgpu adapter");
    let (device, _queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("No wgpu device");

    let result = Model::load_from_obj_bytes(&device, b"not an obj file", "bad.obj");
    assert!(result.is_err());
    let msg = result.err().unwrap().to_string();
    assert!(msg.contains("Model error"), "expected Model error, got: {msg}");
}

#[test]
fn load_texture_raw_no_conversion() {
    let renderer = make_renderer();
    // bingling_sama.png is single-layer (64×32); raw load keeps it as-is.
    let skin = renderer
        .load_texture_raw("resources/bingling_sama.png")
        .expect("load_texture_raw failed");
    // Verify we can render with it (validates GPU resources built correctly).
    let character = Character::new();
    let image = renderer
        .render(&character, &skin, &camera_default(), 200, 150)
        .expect("Render with raw texture failed");
    assert!(image.width() > 0);
}

#[test]
fn render_to_image_no_extension_auto_adjusts() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);
    let tmp = std::env::temp_dir().join("eidolon_auto_ext_noext");
    let tmp_str = tmp.to_str().expect("temp path not UTF-8");
    renderer
        .render_to_image(&character, &skin, &camera_default(), tmp_str, (100, 75), OutputFormat::WebP)
        .expect("render_to_image no-ext failed");
    // Extension adjustment strips directory; file saved to CWD.
    let cwd_file = std::path::Path::new("eidolon_auto_ext_noext.webp");
    assert!(cwd_file.exists(), "expected {cwd_file:?} to exist");
    assert!(std::fs::metadata(cwd_file).unwrap().len() > 0);
    std::fs::remove_file(cwd_file).ok();
}

#[test]
fn camera_extreme_values_produce_finite_matrices() {
    let test_cases = [
        Camera { yaw: 0.0, pitch: 0.0, scale: 0.1 },
        Camera { yaw: 720.0, pitch: -180.0, scale: 10.0 },
        Camera { yaw: 360.0, pitch: 180.0, scale: 0.5 },
        Camera { yaw: -90.0, pitch: 45.0, scale: 2.0 },
    ];
    for cam in &test_cases {
        let view = cam.get_view_matrix();
        for row in &view {
            for &val in row {
                assert!(val.is_finite(), "non-finite view value at yaw={} pitch={} scale={}", cam.yaw, cam.pitch, cam.scale);
            }
        }
        let proj = cam.get_projection_matrix(800, 600);
        for row in &proj {
            for &val in row {
                assert!(val.is_finite(), "non-finite proj value at yaw={} pitch={} scale={}", cam.yaw, cam.pitch, cam.scale);
            }
        }
    }
}

#[test]
fn load_texture_auto_converts_single_layer_skin() {
    // SSSSSteven.png is 64×32 (single-layer, width == 2×height).
    // load_texture must detect this and auto-convert to 64×64 via single2double.
    let renderer = make_renderer();
    let skin = renderer
        .load_texture("resources/SSSSSteven.png")
        .expect("Failed to load single-layer skin");
    let character = Character::new();
    let image = renderer
        .render(&character, &skin, &camera_default(), 200, 150)
        .expect("Render with auto-converted skin failed");
    assert!(image.width() > 0);
}

#[test]
fn load_texture_corrupted_png_errors() {
    let renderer = make_renderer();
    let tmp = std::env::temp_dir().join("eidolon_corrupt_test.png");
    std::fs::write(&tmp, b"this is not a valid PNG image").expect("write temp file");
    let tmp_str = tmp.to_str().expect("temp path not UTF-8");
    let result = renderer.load_texture(tmp_str);
    std::fs::remove_file(&tmp).ok();
    match result {
        Ok(_) => panic!("Expected error for corrupted PNG in load_texture"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("Texture error"),
                "Expected Texture error, got: {msg}"
            );
        }
    }
}

#[test]
fn load_model_from_bytes_missing_required_part_errors() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("No wgpu adapter");
    let (device, _queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("No wgpu device");

    // OBJ with "Head" but missing "Hat Layer" — triggers extract_part error
    let obj_data = b"o Head\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
    let result = Model::load_from_obj_bytes(&device, obj_data, "partial.obj");
    match result {
        Ok(_) => panic!("Expected missing-part error"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("Missing model part"),
                "Expected 'Missing model part' in error, got: {msg}"
            );
        }
    }
}

#[test]
fn render_to_image_png_save_error() {
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);
    // Write to a directory that doesn't exist — save_with_format fails
    let result = renderer.render_to_image(
        &character, &skin, &camera_default(),
        "/nonexistent_dir_xyz_eidolon_test/output.png",
        (100, 75), OutputFormat::Png,
    );
    match result {
        Ok(_) => panic!("Expected error when writing to nonexistent directory"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("failed to save image"),
                "Expected save error, got: {msg}"
            );
        }
    }
}

#[test]
fn depth_texture_cache_hit_on_repeated_same_dimensions() {
    // First render allocates depth texture. Second render with same
    // dimensions must reuse the cached depth texture (cache hit path).
    let renderer = make_renderer();
    let (character, skin) = character_with_skin(&renderer);
    let size = (200, 150);
    let cam = camera_default();

    let img1 = renderer
        .render(&character, &skin, &cam, size.0, size.1)
        .expect("First render failed");
    let img2 = renderer
        .render(&character, &skin, &cam, size.0, size.1)
        .expect("Second render (cache-hit) failed");

    assert_eq!(img1.width(), size.0);
    assert_eq!(img2.width(), size.0);
    assert_eq!(img1.height(), size.1);
    assert_eq!(img2.height(), size.1);
}

#[test]
fn model_switching_slim_and_classic_in_same_renderer() {
    let renderer = make_renderer();
    let skin = renderer
        .load_texture("resources/bingling_sama.png")
        .expect("Failed to load skin");
    let cam = camera_default();

    let mut character = Character::new();

    // Slim
    character.skin_type = SkinType::Slim;
    renderer
        .render(&character, &skin, &cam, 200, 150)
        .expect("Slim render failed");

    // Classic
    character.skin_type = SkinType::Classic;
    renderer
        .render(&character, &skin, &cam, 200, 150)
        .expect("Classic render failed");

    // Back to Slim
    character.skin_type = SkinType::Slim;
    renderer
        .render(&character, &skin, &cam, 200, 150)
        .expect("Slim re-render failed");
}

#[test]
fn render_all_posture_presets_with_both_skin_types() {
    let renderer = make_renderer();
    let skin = renderer
        .load_texture("resources/bingling_sama.png")
        .expect("Failed to load skin");
    let cam = camera_default();
    let postures = [
        DefaultPostures::STAND,
        DefaultPostures::WAVE,
        DefaultPostures::WALKING,
        DefaultPostures::RUNNING,
    ];

    for skin_type in [SkinType::Classic, SkinType::Slim] {
        for (i, posture) in postures.iter().enumerate() {
            let mut character = Character::new();
            character.skin_type = skin_type;
            character.posture = *posture;
            let img = renderer
                .render(&character, &skin, &cam, 160, 120)
                .unwrap_or_else(|e| panic!(
                    "render failed for {:?} posture {}: {e}", skin_type, i
                ));
            assert!(img.width() > 0, "blank render for {:?} posture {}", skin_type, i);
        }
    }
}
