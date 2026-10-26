use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use eidolon_core::{animation::AnimationPreset, character::Posture};

/// Minecraft skin renderer and skin-atlas utilities.
#[derive(Parser, Debug)]
#[command(
    author, version, about, long_about = None,
    after_help = "EXAMPLES:\n  \
                  eidolon render skin.png\n  \
                  eidolon render skin.png out.webp --slim --posture wave\n  \
                  eidolon render skin.png walk.gif --animate --posture walk\n  \
                  eidolon preview skin.png --cam-zoom 2.0\n  \
                  eidolon convert old_skin.png new_skin.png"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq, Eq)]
pub enum PostureCli {
    Idle,
    Walk,
    Run,
    Wave,
    Fly,
}

impl PostureCli {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [Self::Idle, Self::Walk, Self::Run, Self::Wave, Self::Fly];

    pub const fn animation_preset(self) -> AnimationPreset {
        match self {
            PostureCli::Idle => AnimationPreset::Idle,
            PostureCli::Walk => AnimationPreset::Walk,
            PostureCli::Run => AnimationPreset::Run,
            PostureCli::Wave => AnimationPreset::Wave,
            PostureCli::Fly => AnimationPreset::Fly,
        }
    }

    /// Phase used for static rendering and the initial paused preview.
    pub fn sample_time(self) -> std::time::Duration {
        match self {
            Self::Walk | Self::Run => self.animation_preset().cycle_duration() / 4,
            Self::Idle | Self::Wave | Self::Fly => std::time::Duration::ZERO,
        }
    }
}

impl From<PostureCli> for AnimationPreset {
    fn from(value: PostureCli) -> Self {
        value.animation_preset()
    }
}

impl From<PostureCli> for Posture {
    fn from(value: PostureCli) -> Self {
        value.animation_preset().sample(value.sample_time())
    }
}

pub fn parse_positive_scale(s: &str) -> Result<f32, String> {
    let value: f32 = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", s))?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(format!("scale must be greater than 0, got {}", value))
    }
}

pub fn parse_animation_speed(s: &str) -> Result<f32, String> {
    let value: f32 = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if value.is_finite() && (0.25..=4.0).contains(&value) {
        Ok(value)
    } else {
        Err(format!("speed must be between 0.25 and 4.0, got {value}"))
    }
}

#[derive(Parser, Debug)]
pub struct ViewportArgs {
    /// Output image or window width in pixels.
    #[arg(long, default_value_t = 800, value_parser = clap::value_parser!(u32).range(1..))]
    pub width: u32,
    /// Output image or window height in pixels.
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u32).range(1..))]
    pub height: u32,
}

#[derive(Parser, Debug)]
pub struct RenderAnimationArgs {
    /// Render a complete animation cycle instead of a static frame.
    #[arg(long)]
    pub animate: bool,

    /// Animation frames per second.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=60), requires = "animate")]
    pub fps: u32,

    /// Animation playback speed multiplier.
    #[arg(long, default_value_t = 1.0, value_parser = parse_animation_speed, requires = "animate")]
    pub speed: f32,

    /// Animation repetitions in the output metadata; 0 loops forever.
    #[arg(long, default_value_t = 0, requires = "animate")]
    pub loop_count: u16,
}

pub fn resolve_render_output(
    output: Option<String>,
    viewport: &ViewportArgs,
    animation: &RenderAnimationArgs,
) -> Result<String, String> {
    let output = output.unwrap_or_else(|| {
        if animation.animate {
            "output.webp".to_owned()
        } else {
            "output.png".to_owned()
        }
    });

    if !animation.animate {
        return Ok(output);
    }
    if viewport.width > 2_048 || viewport.height > 2_048 {
        return Err("animated output dimensions must not exceed 2048 pixels".to_owned());
    }

    Ok(output)
}

/// Shared scene parameters for render and preview.
#[derive(Parser, Debug)]
pub struct SceneArgs {
    /// Use slim arm geometry (Alex-style, 3px arms). Default is classic (Steve, 4px).
    #[arg(long)]
    pub slim: bool,

    /// Render a Minecraft-style player name above the model.
    #[arg(long)]
    pub nametag: Option<String>,

    /// Hide nametag pixels completely when they are behind scene geometry.
    #[arg(long)]
    pub disable_nametag_see_through: bool,

    /// Render a cape using the specified 64x32 PNG texture.
    #[arg(long, conflicts_with = "elytra")]
    pub cape: Option<String>,

    /// Render elytra using the specified 64x32 PNG texture.
    #[arg(long, conflicts_with = "cape")]
    pub elytra: Option<String>,

    /// Override the posture's cape pitch in degrees.
    #[arg(long)]
    pub cape_pitch: Option<f32>,

    /// Override the posture's elytra pitch in degrees.
    #[arg(long)]
    pub elytra_pitch: Option<f32>,

    /// Override the posture's elytra opening angle in degrees.
    #[arg(long)]
    pub elytra_spread: Option<f32>,

    /// Camera orbit yaw in degrees.
    #[arg(long, default_value_t = 180.0)]
    pub cam_yaw: f32,

    /// Camera orbit pitch in degrees.
    #[arg(long, default_value_t = 90.0)]
    pub cam_pitch: f32,

    /// Camera zoom; higher = larger. Narrows the field of view instead of moving the camera.
    #[arg(long, default_value_t = 1.0, value_parser = parse_positive_scale)]
    pub cam_zoom: f32,

    /// Action posture: idle, walk, run, wave, fly.
    #[arg(long, value_enum, default_value_t = PostureCli::Idle)]
    pub posture: PostureCli,

    // ── per-joint overrides (power-user; shown in --help --long) ──
    #[arg(long, hide_short_help = true)]
    pub head_yaw: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub head_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub left_arm_roll: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub left_arm_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub right_arm_roll: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub right_arm_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub left_leg_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    pub right_leg_pitch: Option<f32>,

    // ── world-space transform (power-user) ──
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pub pos_x: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pub pos_y: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pub pos_z: f32,

    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pub rot_x: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pub rot_y: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pub rot_z: f32,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Render the skin to a static image or animation file (headless).
    ///
    /// Static output accepts PNG or WebP. With --animate, output accepts
    /// GIF or Animated WebP.
    Render {
        /// Path to the skin PNG file.
        skin: String,

        /// Output path. Defaults to output.png, or output.webp with --animate.
        output: Option<String>,

        #[command(flatten)]
        viewport: ViewportArgs,

        #[command(flatten)]
        scene: SceneArgs,

        #[command(flatten)]
        animation: RenderAnimationArgs,
    },
    /// Open a live preview window.
    Preview {
        /// Path to the skin PNG file.
        skin: String,

        #[command(flatten)]
        viewport: ViewportArgs,

        #[command(flatten)]
        scene: SceneArgs,
    },
    /// Convert a legacy single-layer skin atlas to a square double-layer atlas.
    Convert {
        /// Input PNG (width must be twice the height).
        input: PathBuf,
        /// Output PNG path.
        #[arg(default_value = "output.png")]
        output: PathBuf,
    },
    /// Inspect and classify a Minecraft texture (skin, cape, elytra).
    Identify {
        /// Path to the texture PNG image.
        texture: PathBuf,
        /// Output format in JSON.
        #[arg(long)]
        json: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use eidolon_core::output::OutputIntent;

    // ── parse_positive_scale ──

    #[test]
    fn parse_scale_valid() {
        assert_eq!(parse_positive_scale("1.0").unwrap(), 1.0);
        assert_eq!(parse_positive_scale("0.5").unwrap(), 0.5);
        assert_eq!(parse_positive_scale("42").unwrap(), 42.0);
    }

    #[test]
    fn parse_scale_zero_fails() {
        assert!(parse_positive_scale("0").is_err());
        assert!(parse_positive_scale("0.0").is_err());
    }

    #[test]
    fn parse_scale_negative_fails() {
        assert!(parse_positive_scale("-1.0").is_err());
    }

    #[test]
    fn parse_scale_not_a_number_fails() {
        assert!(parse_positive_scale("abc").is_err());
        assert!(parse_positive_scale("").is_err());
    }

    // ── PostureCli → Posture ──

    #[test]
    fn posture_cli_samples_a_representative_animation_frame() {
        for posture in PostureCli::ALL {
            let sampled: Posture = posture.into();
            assert_eq!(
                sampled,
                posture.animation_preset().sample(posture.sample_time())
            );
        }
    }

    #[test]
    fn posture_cli_wave_arms_raised() {
        let p: Posture = PostureCli::Wave.into();
        assert!(p.left_arm_pitch > 0.0);
    }

    #[test]
    fn posture_cli_walk_alternates_limbs() {
        let p: Posture = PostureCli::Walk.into();
        assert!(p.left_arm_pitch < 0.0);
        assert!(p.right_arm_pitch > 0.0);
        assert!(p.left_leg_pitch > 0.0);
        assert!(p.right_leg_pitch < 0.0);
    }

    #[test]
    fn posture_cli_run_is_larger_than_walk() {
        let w: Posture = PostureCli::Walk.into();
        let r: Posture = PostureCli::Run.into();
        assert!(r.left_arm_pitch.abs() > w.left_arm_pitch.abs());
        assert!(r.left_leg_pitch.abs() > w.left_leg_pitch.abs());
    }

    #[test]
    fn posture_cli_fly_spreads_elytra() {
        let posture: Posture = PostureCli::Fly.into();

        assert_eq!(
            posture,
            AnimationPreset::Fly.sample(std::time::Duration::ZERO)
        );
        assert_eq!(posture.body_pitch, -90.0);
        assert!(posture.elytra_roll > 45.0);
    }

    // ── CLI arg parsing ──

    #[test]
    fn cli_render_minimal() {
        let args =
            Args::try_parse_from(["eidolon", "render", "skin.png"]).expect("minimal render parse");
        match args.command {
            Command::Render {
                skin,
                output,
                viewport,
                scene,
                animation,
            } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(output, None);
                assert_eq!(viewport.width, 800);
                assert_eq!(viewport.height, 600);
                assert_eq!(scene.cam_yaw, 180.0);
                assert!(matches!(scene.posture, PostureCli::Idle));
                assert!(!animation.animate);
                assert_eq!(scene.nametag, None);
                assert_eq!(scene.cape, None);
                assert_eq!(scene.elytra, None);
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn cli_render_animate_uses_documented_defaults() {
        let args = Args::try_parse_from(["eidolon", "render", "skin.png", "--animate"])
            .expect("minimal animated render should parse");
        match args.command {
            Command::Render {
                skin,
                output,
                viewport,
                scene,
                animation,
            } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(output, None);
                assert_eq!(scene.posture, PostureCli::Idle);
                assert!(animation.animate);
                assert_eq!(animation.fps, 30);
                assert_eq!(animation.speed, 1.0);
                assert_eq!(animation.loop_count, 0);
                assert_eq!(
                    resolve_render_output(output, &viewport, &animation).unwrap(),
                    "output.webp".to_owned()
                );
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn cli_rejects_removed_animate_command_and_animation_option() {
        assert!(Args::try_parse_from(["eidolon", "animate", "skin.png", "out.webp"]).is_err());
        assert!(Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "out.webp",
            "--animation",
            "run",
        ])
        .is_err());
        assert!(
            Args::try_parse_from(["eidolon", "preview", "skin.png", "--animation", "run",])
                .is_err()
        );
    }

    #[test]
    fn cli_render_animate_selects_its_action_with_posture() {
        let args = Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "out.gif",
            "--animate",
            "--posture",
            "wave",
        ])
        .expect("posture should select the exported action");
        let Command::Render {
            scene, animation, ..
        } = args.command
        else {
            panic!("Expected Render");
        };
        assert!(animation.animate);
        assert_eq!(scene.posture.animation_preset(), AnimationPreset::Wave);
    }

    #[test]
    fn cli_render_animate_validates_format_and_numeric_bounds() {
        let base = [
            "eidolon",
            "render",
            "skin.png",
            "out.gif",
            "--animate",
            "--posture",
            "walk",
        ];
        for invalid_tail in [
            ["--fps", "0"],
            ["--fps", "61"],
            ["--speed", "0.24"],
            ["--speed", "4.01"],
            ["--width", "0"],
            ["--height", "0"],
        ] {
            assert!(Args::try_parse_from(base.into_iter().chain(invalid_tail)).is_err());
        }

        let invalid_output =
            Args::try_parse_from(["eidolon", "render", "skin.png", "out.png", "--animate"])
                .unwrap();
        let Command::Render {
            output,
            viewport,
            animation,
            ..
        } = invalid_output.command
        else {
            panic!("Expected Render");
        };
        let output = resolve_render_output(output, &viewport, &animation).unwrap();
        assert!(OutputIntent::animation_from_path(
            std::path::Path::new(&output),
            AnimationPreset::Walk,
            animation.fps,
            animation.speed,
            animation.loop_count,
            (viewport.width, viewport.height),
        )
        .is_err());

        for valid_ext in ["out.gif", "out.webp", "out.apng", "out.avif"] {
            let valid_output =
                Args::try_parse_from(["eidolon", "render", "skin.png", valid_ext, "--animate"])
                    .unwrap();
            let Command::Render {
                output,
                viewport,
                animation,
                ..
            } = valid_output.command
            else {
                panic!("Expected Render");
            };
            let output = resolve_render_output(output, &viewport, &animation).unwrap();
            assert!(OutputIntent::animation_from_path(
                std::path::Path::new(&output),
                AnimationPreset::Walk,
                animation.fps,
                animation.speed,
                animation.loop_count,
                (viewport.width, viewport.height),
            )
            .is_ok());
        }

        let oversized = Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "out.webp",
            "--animate",
            "--width",
            "2049",
        ])
        .unwrap();
        let Command::Render {
            output,
            viewport,
            animation,
            ..
        } = oversized.command
        else {
            panic!("Expected Render");
        };
        assert!(resolve_render_output(output, &viewport, &animation).is_err());

        assert!(Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "out.webp",
            "--animate",
            "--posture",
            "fly",
            "--fps",
            "60",
            "--speed",
            "4",
            "--width",
            "2048",
            "--height",
            "1",
        ])
        .is_ok());

        assert!(Args::try_parse_from(["eidolon", "render", "skin.png", "--fps", "60",]).is_err());
    }

    #[test]
    fn cli_nametag_is_available_to_render_and_preview() {
        for command in ["render", "preview"] {
            let args = Args::try_parse_from(["eidolon", command, "skin.png", "--nametag", "Steve"])
                .expect("nametag should parse");
            let scene = match args.command {
                Command::Render { scene, .. } | Command::Preview { scene, .. } => scene,
                _ => panic!("Expected render or preview"),
            };

            assert_eq!(scene.nametag.as_deref(), Some("Steve"));
            assert!(!scene.disable_nametag_see_through);
        }

        let args = Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "--nametag",
            "Steve",
            "--disable-nametag-see-through",
        ])
        .expect("nametag see-through toggle should parse");
        match args.command {
            Command::Render { scene, .. } => assert!(scene.disable_nametag_see_through),
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn cli_cape_requires_and_captures_texture_path() {
        assert!(Args::try_parse_from(["eidolon", "render", "skin.png", "--cape"]).is_err());
        assert!(Args::try_parse_from(["eidolon", "preview", "skin.png", "--cape"]).is_err());

        let args = Args::try_parse_from(["eidolon", "render", "skin.png", "--cape", "cape.png"])
            .expect("cape path should parse");
        match args.command {
            Command::Render { scene, .. } => assert_eq!(scene.cape.as_deref(), Some("cape.png")),
            _ => panic!("Expected Render"),
        }

        let args = Args::try_parse_from(["eidolon", "preview", "skin.png", "--cape", "cape.png"])
            .expect("preview cape path should parse");
        match args.command {
            Command::Preview { scene, .. } => assert_eq!(scene.cape.as_deref(), Some("cape.png")),
            _ => panic!("Expected Preview"),
        }
    }

    #[test]
    fn cli_elytra_requires_and_captures_texture_path() {
        assert!(Args::try_parse_from(["eidolon", "render", "skin.png", "--elytra"]).is_err());
        assert!(Args::try_parse_from(["eidolon", "preview", "skin.png", "--elytra"]).is_err());

        for command in ["render", "preview"] {
            let args =
                Args::try_parse_from(["eidolon", command, "skin.png", "--elytra", "elytra.png"])
                    .expect("elytra path should parse");
            let scene = match args.command {
                Command::Render { scene, .. } | Command::Preview { scene, .. } => scene,
                _ => panic!("Expected render or preview"),
            };
            assert_eq!(scene.elytra.as_deref(), Some("elytra.png"));
        }
    }

    #[test]
    fn cli_rejects_cape_and_elytra_together() {
        assert!(Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "--cape",
            "cape.png",
            "--elytra",
            "elytra.png",
        ])
        .is_err());
    }

    #[test]
    fn cli_cape_pitch_is_available_to_render_and_preview() {
        for command in ["render", "preview"] {
            let args = Args::try_parse_from([
                "eidolon",
                command,
                "skin.png",
                "--cape",
                "cape.png",
                "--cape-pitch",
                "25",
            ])
            .expect("cape pitch should parse");
            let scene = match args.command {
                Command::Render { scene, .. } | Command::Preview { scene, .. } => scene,
                _ => panic!("Expected render or preview"),
            };
            assert_eq!(scene.cape_pitch, Some(25.0));
        }
    }

    #[test]
    fn cli_elytra_angles_override_the_selected_posture() {
        for command in ["render", "preview"] {
            let args = Args::try_parse_from([
                "eidolon",
                command,
                "skin.png",
                "--posture",
                "fly",
                "--elytra-pitch",
                "18",
                "--elytra-spread",
                "42",
            ])
            .expect("elytra angles should parse");
            let scene = match args.command {
                Command::Render { scene, .. } | Command::Preview { scene, .. } => scene,
                _ => panic!("Expected render or preview"),
            };
            assert_eq!(scene.elytra_pitch, Some(18.0));
            assert_eq!(scene.elytra_spread, Some(42.0));

            let (state, _) = crate::scene::scene_state_and_camera_from_scene(&scene);
            assert_eq!(state.posture.elytra_pitch, 18.0);
            assert_eq!(state.posture.elytra_roll, 42.0);
        }
    }

    #[test]
    fn cli_render_all_options() {
        let args = Args::try_parse_from([
            "eidolon",
            "render",
            "skin.png",
            "out.webp",
            "--width",
            "400",
            "--height",
            "300",
            "--slim",
            "--cam-yaw",
            "90",
            "--cam-pitch",
            "45",
            "--cam-zoom",
            "2.0",
            "--posture",
            "wave",
            "--head-yaw",
            "15",
            "--left-arm-pitch",
            "45",
            "--pos-x",
            "1",
            "--rot-y",
            "180",
        ])
        .expect("full render parse");
        match args.command {
            Command::Render {
                skin,
                output,
                viewport,
                scene,
                animation,
            } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(output.as_deref(), Some("out.webp"));
                assert!(!animation.animate);
                assert_eq!(viewport.width, 400);
                assert_eq!(viewport.height, 300);
                assert!(scene.slim);
                assert_eq!(scene.cam_yaw, 90.0);
                assert_eq!(scene.cam_pitch, 45.0);
                assert_eq!(scene.cam_zoom, 2.0);
                assert!(matches!(scene.posture, PostureCli::Wave));
                assert_eq!(scene.head_yaw, Some(15.0));
                assert_eq!(scene.left_arm_pitch, Some(45.0));
                assert_eq!(scene.pos_x, 1.0);
                assert_eq!(scene.rot_y, 180.0);
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn cli_render_invalid_width_rejected() {
        assert!(Args::try_parse_from(["eidolon", "render", "skin.png", "--width", "0"]).is_err());
    }

    #[test]
    fn cli_render_invalid_zoom_zero_rejected() {
        assert!(
            Args::try_parse_from(["eidolon", "render", "skin.png", "--cam-zoom", "0"]).is_err()
        );
    }

    #[test]
    fn cli_render_invalid_zoom_negative_rejected() {
        assert!(
            Args::try_parse_from(["eidolon", "render", "skin.png", "--cam-zoom", "-1"]).is_err()
        );
    }

    #[test]
    fn cli_render_invalid_posture_rejected() {
        assert!(
            Args::try_parse_from(["eidolon", "render", "skin.png", "--posture", "unknown"])
                .is_err()
        );
    }

    #[test]
    fn cli_posture_names_exactly_match_animation_presets() {
        for name in ["idle", "walk", "run", "wave", "fly"] {
            assert!(
                Args::try_parse_from(["eidolon", "render", "skin.png", "--posture", name]).is_ok(),
                "{name} should be accepted"
            );
        }
        for legacy_name in ["stand", "walking", "running", "flying"] {
            assert!(
                Args::try_parse_from(["eidolon", "render", "skin.png", "--posture", legacy_name,])
                    .is_err(),
                "{legacy_name} should be rejected"
            );
        }
    }

    #[test]
    fn cli_preview_minimal() {
        let args = Args::try_parse_from(["eidolon", "preview", "skin.png"])
            .expect("minimal preview parse");
        match args.command {
            Command::Preview {
                skin,
                viewport,
                scene,
                ..
            } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(viewport.width, 800);
                assert_eq!(viewport.height, 600);
                assert!(!scene.slim);
            }
            _ => panic!("Expected Preview"),
        }
    }

    #[test]
    fn cli_preview_with_options() {
        let args = Args::try_parse_from([
            "eidolon",
            "preview",
            "skin.png",
            "--width",
            "1024",
            "--height",
            "768",
            "--slim",
            "--posture",
            "run",
            "--cam-zoom",
            "1.5",
        ])
        .expect("preview with options parse");
        match args.command {
            Command::Preview {
                skin,
                viewport,
                scene,
                ..
            } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(viewport.width, 1024);
                assert_eq!(viewport.height, 768);
                assert!(scene.slim);
                assert!(matches!(scene.posture, PostureCli::Run));
                assert_eq!(scene.cam_zoom, 1.5);
            }
            _ => panic!("Expected Preview"),
        }
    }

    #[test]
    fn cli_preview_selects_its_action_with_posture() {
        let args = Args::try_parse_from(["eidolon", "preview", "skin.png", "--posture", "run"])
            .expect("posture should select the preview action");
        let Command::Preview { scene, .. } = args.command else {
            panic!("Expected Preview");
        };
        assert_eq!(scene.posture.animation_preset(), AnimationPreset::Run);
    }

    #[test]
    fn cli_convert_minimal() {
        let args = Args::try_parse_from(["eidolon", "convert", "old.png", "new.png"])
            .expect("minimal convert parse");
        match args.command {
            Command::Convert { input, output } => {
                assert_eq!(input, PathBuf::from("old.png"));
                assert_eq!(output, PathBuf::from("new.png"));
            }
            _ => panic!("Expected Convert"),
        }
    }

    #[test]
    fn cli_convert_default_output() {
        let args = Args::try_parse_from(["eidolon", "convert", "old.png"])
            .expect("convert with default output");
        match args.command {
            Command::Convert { input, output } => {
                assert_eq!(input, PathBuf::from("old.png"));
                assert_eq!(output, PathBuf::from("output.png"));
            }
            _ => panic!("Expected Convert"),
        }
    }

    #[test]
    fn cli_missing_subcommand_rejected() {
        assert!(Args::try_parse_from(["eidolon"]).is_err());
    }

    #[test]
    fn cli_render_missing_skin_rejected() {
        assert!(Args::try_parse_from(["eidolon", "render"]).is_err());
    }

    #[test]
    fn cli_render_help_accepted() {
        // --help should print and exit
        let result = Args::try_parse_from(["eidolon", "render", "--help"]);
        assert!(result.is_err()); // clap exits on help by default
    }
}
