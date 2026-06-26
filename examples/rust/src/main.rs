//! Eidolon library usage example.
//!
//! Renders a skin with several posture/format/skin-type combinations to show
//! the public API.  Run with `cargo run` from this directory, or from the
//! repo root with `cargo run -p eidolon-example`.
//!
//! Output lands in `examples/rust/out/`.

use std::path::{Path, PathBuf};

use eidolon::{
    camera::Camera,
    character::{Character, DefaultPostures, Posture, SkinType},
    renderer::{OutputFormat, Renderer},
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
    let skin = renderer.load_texture("resources/bingling_sama.png")?;

    // ── helper: render + save ────────────────────────────────────────
    let save = |name: &str,
                character: &Character,
                camera: &Camera,
                format: OutputFormat| -> Result<(), Box<dyn std::error::Error>> {
        let path = out_dir.join(name);
        renderer.render_to_image(
            character,
            &skin,
            camera,
            &path.display().to_string(),
            (800, 600),
            format,
        )?;
        println!("  -> {}", path.display());
        Ok(())
    };

    // ── default camera ───────────────────────────────────────────────
    let default_cam = Camera::new();

    // ── 1. classic + stand (PNG) ─────────────────────────────────────
    println!("classic-stand");
    let mut character = Character::new();
    character.skin_type = SkinType::Slim;
    character.posture = DefaultPostures::STAND;
    save("classic-stand.png", &character, &default_cam, OutputFormat::Png)?;

    // ── 2. classic + wave (WebP) ─────────────────────────────────────
    println!("classic-wave");
    character.posture = DefaultPostures::WAVE;
    save(
        "classic-wave.webp",
        &character,
        &default_cam,
        OutputFormat::WebP,
    )?;

    // ── 3. slim + walking, angled camera ─────────────────────────────
    println!("slim-walking");
    character.skin_type = SkinType::Slim;
    character.posture = DefaultPostures::WALKING;
    let cam = Camera {
        yaw: 200.0,
        pitch: 95.0,
        scale: 1.2,
    };
    save("slim-walking.png", &character, &cam, OutputFormat::Png)?;

    // ── 4. slim + running, zoomed out ────────────────────────────────
    println!("slim-running");
    character.posture = DefaultPostures::RUNNING;
    let cam = Camera {
        yaw: 160.0,
        pitch: 85.0,
        scale: 0.8,
    };
    save("slim-running.png", &character, &cam, OutputFormat::Png)?;

    // ── 5. custom posture (per-joint) ────────────────────────────────
    println!("custom-posture");
    character.posture = Posture {
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
    save("custom-posture.png", &character, &cam, OutputFormat::Png)?;

    println!("\nDone — {} images in {}", 5, out_dir.display());
    Ok(())
}
