//! Eidolon library usage example.
//!
//! Renders a skin with posture/format/skin-type/back-equipment combinations to show
//! the public API.  Run with `cargo run` from this directory, or from the
//! repo root with `cargo run -p eidolon-example`.
//!
//! Output lands in `examples/rust/out/`.

use std::path::{Path, PathBuf};

use eidolon::{
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, DefaultPostures, Posture, SkinType},
    output::{write_output, OutputIntent, StaticFormat},
    renderer::Renderer,
};

/// Repo root, derived from `CARGO_MANIFEST_DIR` (works via `cargo run`).
fn repo_root() -> PathBuf {
    let p = if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // CARGO_MANIFEST_DIR points to examples/rust/ — go up two levels.
        Path::new(&dir).join("../..")
    } else {
        // Fallback: assume CWD is already the repo root.
        PathBuf::from(".")
    };
    p.canonicalize().unwrap_or(p)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let root = repo_root();
    // Renderer loads OBJs from CWD-relative paths ("resources/slim.obj").
    std::env::set_current_dir(&root)?;

    let out_dir = root.join("examples/rust/out");
    std::fs::create_dir_all(&out_dir)?;

    // ── one-time setup: create renderer + load skin ──────────────────
    let renderer = Renderer::new()?;
    let body = renderer.load_body("resources/bingling_sama.png", SkinType::Classic)?;
    let back_equipment_atlas = renderer.load_back_equipment_atlas("resources/Turtle_Cape.png")?;

    // ── helper: render + save ────────────────────────────────────────
    let save = |name: &str, character: &Character, camera: &Camera, format: StaticFormat| {
        let path = out_dir.join(name);
        let written = write_output(
            &path,
            OutputIntent::Static {
                format,
                size: (800, 600),
            },
            |_: std::time::Duration| renderer.render(character, camera, 800, 600),
        )?;
        println!("  -> {}", written.display());
        Ok::<(), eidolon::error::EidolonError>(())
    };

    // ── default camera ───────────────────────────────────────────────
    let default_cam = Camera::new();

    // ── 1. classic + stand (PNG) ─────────────────────────────────────
    println!("classic-stand");
    let mut character = Character::from_body(body);
    character.posture = DefaultPostures::STAND;
    save(
        "classic-stand.png",
        &character,
        &default_cam,
        StaticFormat::Png,
    )?;

    // ── 2. classic + wave (WebP) ─────────────────────────────────────
    println!("classic-wave");
    character.posture = DefaultPostures::WAVE;
    save(
        "classic-wave.webp",
        &character,
        &default_cam,
        StaticFormat::WebP,
    )?;

    // ── 3. slim + walking, angled camera ─────────────────────────────
    println!("slim-walking");
    character
        .body_mut()
        .expect("character has a body")
        .set_skin_type(SkinType::Slim);
    character.posture = DefaultPostures::WALKING;
    let cam = Camera {
        yaw: 200.0,
        pitch: 95.0,
        scale: 1.2,
    };
    save("slim-walking.png", &character, &cam, StaticFormat::Png)?;

    // ── 4. slim + running, zoomed out ────────────────────────────────
    println!("slim-running");
    character.posture = DefaultPostures::RUNNING;
    let cam = Camera {
        yaw: 160.0,
        pitch: 85.0,
        scale: 0.8,
    };
    save("slim-running.png", &character, &cam, StaticFormat::Png)?;

    // ── 5. slim + running + cape, rear camera ────────────────────────
    println!("slim-running-cape");
    character.equip(BackEquipment::new(
        back_equipment_atlas,
        BackEquipmentKind::Cape,
    ));
    let cam = Camera {
        yaw: 0.0,
        pitch: 90.0,
        scale: 0.9,
    };
    save("slim-running-cape.png", &character, &cam, StaticFormat::Png)?;

    // ── 6. custom posture (per-joint) + cape ─────────────────────────
    println!("custom-posture");
    character.posture = Posture {
        cape_pitch: 10.8,
        body_pitch: 0.0,
        elytra_pitch: 15.0,
        elytra_yaw: 0.5729578,
        elytra_roll: 15.0,
        head_yaw: 30.0,
        head_pitch: -10.0,
        left_arm_roll: 45.0,
        left_arm_pitch: -90.0,
        right_arm_roll: -45.0,
        right_arm_pitch: 90.0,
        left_leg_pitch: 20.0,
        right_leg_pitch: -20.0,
    };
    let cam = Camera {
        yaw: 180.0,
        pitch: 90.0,
        scale: 1.0,
    };
    save("custom-posture.png", &character, &cam, StaticFormat::Png)?;

    // -- 7. elytra, rear camera ---------------------------------------
    println!("slim-elytra");
    character
        .back_equipment_mut()
        .expect("character has back equipment")
        .set_kind(BackEquipmentKind::Elytra);
    character.posture = DefaultPostures::FLYING;
    let rear_cam = Camera { yaw: 0.0, ..cam };
    save("slim-elytra.png", &character, &rear_cam, StaticFormat::Png)?;

    println!("\nDone — {} images in {}", 7, out_dir.display());
    Ok(())
}
