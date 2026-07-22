//! Integration tests for the eidolon renderer.
//!
//! Exercise the full public API: Character → Renderer → ImageBuffer → file.
//! Headless via OSMesa — no GPU required.

#![cfg(not(target_arch = "wasm32"))]

use eidolon::back_equipment::{BackEquipment, BackEquipmentKind};
use eidolon::camera::Camera;
use eidolon::character::{Character, CharacterError, DefaultPostures, Posture, SkinType};
use eidolon::model::Model;
use eidolon::output::{write_output, OutputIntent, StaticFormat};
use eidolon::renderer::Renderer;

fn make_renderer() -> Renderer {
    Renderer::new().expect("Failed to create Renderer")
}

fn character_with_skin(renderer: &Renderer) -> Character {
    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("Failed to load body");
    let mut c = Character::from_body(body);
    c.posture = Posture {
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
    c
}

fn camera_default() -> Camera {
    Camera {
        yaw: 180.0,
        pitch: 90.0,
        scale: 1.0,
    }
}

#[test]
fn cape_can_render_without_body() {
    let renderer = make_renderer();
    let atlas = renderer
        .load_back_equipment_atlas("resources/Turtle_Cape.png")
        .expect("load back equipment atlas failed");
    let character =
        Character::from_back_equipment(BackEquipment::new(atlas, BackEquipmentKind::Cape));

    let image = renderer
        .render(&character, &camera_default(), 200, 150)
        .expect("standalone cape render failed");

    assert!(
        image.pixels().any(|pixel| pixel[3] > 0),
        "standalone cape should produce visible pixels"
    );
}

#[test]
fn elytra_can_render_without_body() {
    let renderer = make_renderer();
    let atlas = renderer
        .load_back_equipment_atlas("resources/Turtle_Cape.png")
        .expect("load back equipment atlas failed");
    let character =
        Character::from_back_equipment(BackEquipment::new(atlas, BackEquipmentKind::Elytra));

    let image = renderer
        .render(&character, &camera_default(), 200, 150)
        .expect("standalone elytra render failed");

    assert!(
        image.pixels().any(|pixel| pixel[3] > 0),
        "standalone elytra should produce visible pixels"
    );
}

#[test]
fn removing_the_only_visible_resource_is_rejected() {
    let renderer = make_renderer();
    let atlas = renderer
        .load_back_equipment_atlas("resources/Turtle_Cape.png")
        .expect("load back equipment atlas failed");
    let mut character =
        Character::from_back_equipment(BackEquipment::new(atlas, BackEquipmentKind::Cape));

    assert!(matches!(
        character.unequip(),
        Err(CharacterError::WouldBecomeEmpty)
    ));
    assert!(character.back_equipment().is_some());
}

#[test]
fn removing_a_missing_resource_is_idempotent() {
    let renderer = make_renderer();
    let atlas = renderer
        .load_back_equipment_atlas("resources/Turtle_Cape.png")
        .expect("load back equipment atlas failed");
    let mut equipment_only =
        Character::from_back_equipment(BackEquipment::new(atlas, BackEquipmentKind::Cape));
    assert!(equipment_only
        .remove_body()
        .expect("remove body failed")
        .is_none());

    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("load body failed");
    let mut body_only = Character::from_body(body);
    assert!(body_only.unequip().expect("unequip failed").is_none());
}

#[test]
fn removing_the_only_body_is_rejected() {
    let renderer = make_renderer();
    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("load body failed");
    let mut character = Character::from_body(body);

    assert!(matches!(
        character.remove_body(),
        Err(CharacterError::WouldBecomeEmpty)
    ));
    assert!(character.body().is_some());
}

#[test]
fn body_is_owned_by_character_and_renders() {
    let renderer = make_renderer();
    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("load body failed");
    let character = Character::from_body(body);

    let image = renderer
        .render(&character, &camera_default(), 200, 150)
        .expect("body render failed");

    assert!(image.pixels().any(|pixel| pixel[3] > 0));
}

#[test]
fn cloned_character_reuses_resources_with_the_same_renderer() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);
    let cloned = character.clone();

    renderer
        .render(&cloned, &camera_default(), 200, 150)
        .expect("cloned character should render in its resource context");
}

#[test]
fn character_resources_cannot_cross_renderer_contexts() {
    let source_renderer = make_renderer();
    let target_renderer = make_renderer();
    let body = source_renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("load body failed");
    let character = Character::from_body(body);

    let error = target_renderer
        .render(&character, &camera_default(), 200, 150)
        .expect_err("cross-context render should fail");

    assert!(error.to_string().contains("different renderer context"));
}

#[test]
fn render_produces_correct_dimensions() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);
    let image = renderer
        .render(&character, &camera_default(), 800, 600)
        .expect("Render failed");
    assert_eq!(image.width(), 800);
    assert_eq!(image.height(), 600);
}

#[test]
fn render_output_not_blank() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);
    let image = renderer
        .render(&character, &camera_default(), 800, 600)
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
fn nametag_renders_above_the_player() {
    let renderer = make_renderer();
    let mut character = character_with_skin(&renderer);
    let without_nametag = renderer
        .render(&character, &camera_default(), 400, 300)
        .expect("render without nametag failed");

    character.nametag = Some("Steve".to_owned());
    let with_nametag = renderer
        .render(&character, &camera_default(), 400, 300)
        .expect("render with nametag failed");

    let changed_pixels: Vec<(u32, u32)> = without_nametag
        .enumerate_pixels()
        .zip(with_nametag.enumerate_pixels())
        .filter_map(|((x, y, before), (_, _, after))| (before != after).then_some((x, y)))
        .collect();
    assert!(
        changed_pixels.len() > 20,
        "nametag should change visible pixels"
    );
    assert!(
        changed_pixels.iter().all(|(_, y)| *y < 150),
        "nametag changes should stay above the player's midpoint"
    );
}

#[test]
fn nametag_see_through_only_reveals_occluded_pixels() {
    let renderer = make_renderer();
    let mut character = character_with_skin(&renderer);
    character.posture = DefaultPostures::FLYING;
    let back_camera = Camera {
        yaw: 0.0,
        pitch: 30.0,
        scale: 1.0,
    };
    let without_nametag = renderer
        .render(&character, &back_camera, 400, 300)
        .expect("render without nametag failed");

    character.nametag = Some("BinglingTX".to_owned());
    character.nametag_see_through = false;
    let fully_occluded = renderer
        .render(&character, &back_camera, 400, 300)
        .expect("render with full nametag occlusion failed");

    character.nametag_see_through = true;
    let translucent_occlusion = renderer
        .render(&character, &back_camera, 400, 300)
        .expect("render with translucent nametag occlusion failed");

    let revealed_pixels = without_nametag
        .pixels()
        .zip(fully_occluded.pixels())
        .zip(translucent_occlusion.pixels())
        .filter(|((without, hidden), translucent)| without == hidden && without != translucent)
        .count();
    assert!(
        revealed_pixels > 10,
        "see-through pass should reveal nametag pixels hidden by the player model"
    );
}

#[test]
fn different_camera_angle_produces_different_output() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);

    let front = renderer
        .render(
            &character,
            &Camera {
                yaw: 90.0,
                pitch: 90.0,
                scale: 1.0,
            },
            200,
            150,
        )
        .expect("Front render failed");
    let back = renderer
        .render(
            &character,
            &Camera {
                yaw: 270.0,
                pitch: 90.0,
                scale: 1.0,
            },
            200,
            150,
        )
        .expect("Back render failed");

    let px_front: Vec<u8> = front.pixels().flat_map(|p| p.0.to_vec()).collect();
    let px_back: Vec<u8> = back.pixels().flat_map(|p| p.0.to_vec()).collect();
    assert_ne!(px_front, px_back, "Front and back renders must differ");
}

#[test]
fn enabling_elytra_changes_the_back_view() {
    let renderer = make_renderer();
    let mut character = character_with_skin(&renderer);
    let back_camera = Camera {
        yaw: 0.0,
        pitch: 90.0,
        scale: 1.0,
    };

    let without_elytra = renderer
        .render(&character, &back_camera, 200, 150)
        .expect("render without elytra failed");

    let atlas = renderer
        .load_back_equipment_atlas("resources/Turtle_Cape.png")
        .expect("load back equipment atlas failed");
    character.equip(BackEquipment::new(atlas, BackEquipmentKind::Elytra));
    let with_elytra = renderer
        .render(&character, &back_camera, 200, 150)
        .expect("render with elytra failed");

    assert_ne!(without_elytra.as_raw(), with_elytra.as_raw());
}

#[test]
fn flying_posture_changes_the_elytra_back_view() {
    let renderer = make_renderer();
    let mut character = character_with_skin(&renderer);
    let back_camera = Camera {
        yaw: 0.0,
        pitch: 90.0,
        scale: 1.0,
    };
    let atlas = renderer
        .load_back_equipment_atlas("resources/Turtle_Cape.png")
        .expect("load back equipment atlas failed");
    character.equip(BackEquipment::new(atlas, BackEquipmentKind::Elytra));

    character.posture = DefaultPostures::STAND;
    let folded = renderer
        .render(&character, &back_camera, 200, 150)
        .expect("render folded elytra failed");
    character.posture = DefaultPostures::FLYING;
    let flying = renderer
        .render(&character, &back_camera, 200, 150)
        .expect("render flying elytra failed");

    assert_ne!(folded.as_raw(), flying.as_raw());
}

#[test]
fn elytra_rejects_non_cape_atlas_dimensions() {
    let renderer = make_renderer();
    let error = renderer
        .load_back_equipment_atlas("resources/bingling_sama.png")
        .expect_err("square skin must not be accepted as an elytra atlas");

    assert!(error.to_string().contains("64x32"));
}

#[test]
fn output_saves_static_png_to_disk() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);

    let tmp = std::env::temp_dir().join("eidolon_test_output.png");
    let resolved = write_output(
        &tmp,
        OutputIntent::Static {
            format: StaticFormat::Png,
            size: (200, 150),
        },
        |_: std::time::Duration| renderer.render(&character, &camera_default(), 200, 150),
    )
    .expect("output failed");

    assert_eq!(resolved, tmp);
    assert!(resolved.exists(), "Output file not created: {:?}", resolved);
    assert!(
        std::fs::metadata(&resolved).unwrap().len() > 0,
        "Output file empty"
    );
    std::fs::remove_file(&resolved).ok();
}

#[test]
fn output_writes_static_webp() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);

    let tmp = std::env::temp_dir().join("eidolon_test_output.webp");
    let resolved = write_output(
        &tmp,
        OutputIntent::Static {
            format: StaticFormat::WebP,
            size: (200, 150),
        },
        |_: std::time::Duration| renderer.render(&character, &camera_default(), 200, 150),
    )
    .expect("output failed");

    assert_eq!(resolved, tmp);
    assert!(resolved.exists(), "WebP output file not created");
    assert!(
        std::fs::metadata(&resolved).unwrap().len() > 0,
        "WebP output empty"
    );
    std::fs::remove_file(&resolved).ok();
}

#[test]
fn load_body_invalid_path_returns_error() {
    let renderer = make_renderer();
    let result = renderer.load_body("nonexistent_skin_xyz.png", SkinType::Classic);
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

    let from_file =
        Model::load_from_obj(&device, "resources/slim.obj").expect("load_from_obj slim failed");
    let from_bytes = Model::load_from_obj_bytes(&device, &slim_bytes, "slim.obj")
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
    assert!(
        msg.contains("Model error"),
        "expected Model error, got: {msg}"
    );
}

#[test]
fn output_extension_adjustment_preserves_parent_directory() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);
    let tmp = std::env::temp_dir().join("eidolon_auto_ext_noext");
    let resolved = write_output(
        &tmp,
        OutputIntent::Static {
            format: StaticFormat::WebP,
            size: (100, 75),
        },
        |_: std::time::Duration| renderer.render(&character, &camera_default(), 100, 75),
    )
    .expect("output extension adjustment failed");
    assert_eq!(resolved, tmp.with_extension("webp"));
    assert!(resolved.exists(), "expected {resolved:?} to exist");
    assert!(std::fs::metadata(&resolved).unwrap().len() > 0);
    std::fs::remove_file(resolved).ok();
}

#[test]
fn camera_extreme_values_produce_finite_matrices() {
    let test_cases = [
        Camera {
            yaw: 0.0,
            pitch: 0.0,
            scale: 0.1,
        },
        Camera {
            yaw: 720.0,
            pitch: -180.0,
            scale: 10.0,
        },
        Camera {
            yaw: 360.0,
            pitch: 180.0,
            scale: 0.5,
        },
        Camera {
            yaw: -90.0,
            pitch: 45.0,
            scale: 2.0,
        },
    ];
    for cam in &test_cases {
        let view = cam.get_view_matrix();
        for row in &view {
            for &val in row {
                assert!(
                    val.is_finite(),
                    "non-finite view value at yaw={} pitch={} scale={}",
                    cam.yaw,
                    cam.pitch,
                    cam.scale
                );
            }
        }
        let proj = cam.get_projection_matrix(800, 600);
        for row in &proj {
            for &val in row {
                assert!(
                    val.is_finite(),
                    "non-finite proj value at yaw={} pitch={} scale={}",
                    cam.yaw,
                    cam.pitch,
                    cam.scale
                );
            }
        }
    }
}

#[test]
fn load_body_auto_converts_single_layer_skin() {
    // SSSSSteven.png is 64×32 (single-layer, width == 2×height).
    // load_body must detect this and auto-convert to 64×64 via single2double.
    let renderer = make_renderer();
    let body = renderer
        .load_body("resources/SSSSSteven.png", SkinType::Classic)
        .expect("Failed to load single-layer skin");
    let character = Character::from_body(body);
    let image = renderer
        .render(&character, &camera_default(), 200, 150)
        .expect("Render with auto-converted skin failed");
    assert!(image.width() > 0);
}

#[test]
fn load_body_corrupted_png_errors() {
    let renderer = make_renderer();
    let tmp = std::env::temp_dir().join("eidolon_corrupt_test.png");
    std::fs::write(&tmp, b"this is not a valid PNG image").expect("write temp file");
    let tmp_str = tmp.to_str().expect("temp path not UTF-8");
    let result = renderer.load_body(tmp_str, SkinType::Classic);
    std::fs::remove_file(&tmp).ok();
    match result {
        Ok(_) => panic!("Expected error for corrupted PNG in load_body"),
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
fn output_reports_static_save_error() {
    let renderer = make_renderer();
    let character = character_with_skin(&renderer);
    let result = write_output(
        "/nonexistent_dir_xyz_eidolon_test/output.png",
        OutputIntent::Static {
            format: StaticFormat::Png,
            size: (100, 75),
        },
        |_: std::time::Duration| renderer.render(&character, &camera_default(), 100, 75),
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
    let character = character_with_skin(&renderer);
    let size = (200, 150);
    let cam = camera_default();

    let img1 = renderer
        .render(&character, &cam, size.0, size.1)
        .expect("First render failed");
    let img2 = renderer
        .render(&character, &cam, size.0, size.1)
        .expect("Second render (cache-hit) failed");

    assert_eq!(img1.width(), size.0);
    assert_eq!(img2.width(), size.0);
    assert_eq!(img1.height(), size.1);
    assert_eq!(img2.height(), size.1);
}

#[test]
fn model_switching_slim_and_classic_in_same_renderer() {
    let renderer = make_renderer();
    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("Failed to load body");
    let cam = camera_default();
    let mut character = Character::from_body(body);

    // Slim
    character
        .body_mut()
        .expect("character should have a body")
        .set_skin_type(SkinType::Slim);
    renderer
        .render(&character, &cam, 200, 150)
        .expect("Slim render failed");

    // Classic
    character
        .body_mut()
        .expect("character should have a body")
        .set_skin_type(SkinType::Classic);
    renderer
        .render(&character, &cam, 200, 150)
        .expect("Classic render failed");

    // Back to Slim
    character
        .body_mut()
        .expect("character should have a body")
        .set_skin_type(SkinType::Slim);
    renderer
        .render(&character, &cam, 200, 150)
        .expect("Slim re-render failed");
}

#[test]
fn render_all_posture_presets_with_both_skin_types() {
    let renderer = make_renderer();
    let body = renderer
        .load_body("resources/bingling_sama.png", SkinType::Classic)
        .expect("Failed to load body");
    let cam = camera_default();
    let postures = [
        DefaultPostures::STAND,
        DefaultPostures::WAVE,
        DefaultPostures::WALKING,
        DefaultPostures::RUNNING,
    ];

    for skin_type in [SkinType::Classic, SkinType::Slim] {
        for (i, posture) in postures.iter().enumerate() {
            let mut character = Character::from_body(body.clone());
            character
                .body_mut()
                .expect("character should have a body")
                .set_skin_type(skin_type);
            character.posture = *posture;
            let img = renderer
                .render(&character, &cam, 160, 120)
                .unwrap_or_else(|e| panic!("render failed for {:?} posture {}: {e}", skin_type, i));
            assert!(
                img.width() > 0,
                "blank render for {:?} posture {}",
                skin_type,
                i
            );
        }
    }
}
