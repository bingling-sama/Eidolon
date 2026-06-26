use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use eidolon::{
    camera::Camera,
    character::{Character, DefaultPostures, Posture, SkinType},
    converter,
    renderer::{OutputFormat, Renderer},
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Minecraft skin renderer and skin-atlas utilities.
#[derive(Parser, Debug)]
#[command(
    author, version, about, long_about = None,
    after_help = "EXAMPLES:\n  \
                  eidolon render skin.png\n  \
                  eidolon render skin.png out.webp --slim --posture wave\n  \
                  eidolon preview skin.png --cam-zoom 2.0\n  \
                  eidolon convert old_skin.png new_skin.png"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum PostureCli {
    Stand,
    Wave,
    Walking,
    Running,
}

impl From<PostureCli> for Posture {
    fn from(value: PostureCli) -> Self {
        match value {
            PostureCli::Stand => DefaultPostures::STAND,
            PostureCli::Wave => DefaultPostures::WAVE,
            PostureCli::Walking => DefaultPostures::WALKING,
            PostureCli::Running => DefaultPostures::RUNNING,
        }
    }
}

fn parse_positive_scale(s: &str) -> Result<f32, String> {
    let value: f32 = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", s))?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(format!("scale must be greater than 0, got {}", value))
    }
}

#[derive(Parser, Debug)]
struct ViewportArgs {
    /// Output image or window width in pixels.
    #[arg(long, default_value_t = 800, value_parser = clap::value_parser!(u32).range(1..))]
    width: u32,
    /// Output image or window height in pixels.
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u32).range(1..))]
    height: u32,
}

/// Shared scene parameters for render and preview.
#[derive(Parser, Debug)]
struct SceneArgs {
    /// Use slim arm geometry (Alex-style, 3px arms). Default is classic (Steve, 4px).
    #[arg(long)]
    slim: bool,

    /// Camera orbit yaw in degrees.
    #[arg(long, default_value_t = 180.0)]
    cam_yaw: f32,

    /// Camera orbit pitch in degrees.
    #[arg(long, default_value_t = 90.0)]
    cam_pitch: f32,

    /// Camera zoom; higher = closer (orbit radius: 4.0 / zoom).
    #[arg(long, default_value_t = 1.0, value_parser = parse_positive_scale)]
    cam_zoom: f32,

    /// Posture preset: stand, wave, walking, running.
    #[arg(long, value_enum, default_value_t = PostureCli::Stand)]
    posture: PostureCli,

    // ── per-joint overrides (power-user; shown in --help --long) ──
    #[arg(long, hide_short_help = true)]
    head_yaw: Option<f32>,
    #[arg(long, hide_short_help = true)]
    head_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    left_arm_roll: Option<f32>,
    #[arg(long, hide_short_help = true)]
    left_arm_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    right_arm_roll: Option<f32>,
    #[arg(long, hide_short_help = true)]
    right_arm_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    left_leg_pitch: Option<f32>,
    #[arg(long, hide_short_help = true)]
    right_leg_pitch: Option<f32>,

    // ── world-space transform (power-user) ──
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pos_x: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pos_y: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    pos_z: f32,

    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    rot_x: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    rot_y: f32,
    #[arg(long, hide_short_help = true, default_value_t = 0.0)]
    rot_z: f32,
}

fn character_and_camera_from_scene(scene: &SceneArgs) -> (Character, Camera) {
    let mut character = Character::new();
    character.skin_type = if scene.slim {
        SkinType::Slim
    } else {
        SkinType::Classic
    };
    let base: Posture = scene.posture.into();
    character.posture = Posture {
        head_yaw: scene.head_yaw.unwrap_or(base.head_yaw),
        head_pitch: scene.head_pitch.unwrap_or(base.head_pitch),
        left_arm_roll: scene.left_arm_roll.unwrap_or(base.left_arm_roll),
        left_arm_pitch: scene.left_arm_pitch.unwrap_or(base.left_arm_pitch),
        right_arm_roll: scene.right_arm_roll.unwrap_or(base.right_arm_roll),
        right_arm_pitch: scene.right_arm_pitch.unwrap_or(base.right_arm_pitch),
        left_leg_pitch: scene.left_leg_pitch.unwrap_or(base.left_leg_pitch),
        right_leg_pitch: scene.right_leg_pitch.unwrap_or(base.right_leg_pitch),
    };
    character.position =
        cgmath::Vector3::new(scene.pos_x, scene.pos_y, scene.pos_z);
    character.rotation =
        cgmath::Vector3::new(scene.rot_x, scene.rot_y, scene.rot_z);

    let camera = Camera {
        yaw: scene.cam_yaw,
        pitch: scene.cam_pitch,
        scale: scene.cam_zoom,
    };
    (character, camera)
}

/// Infer OutputFormat from filename extension. Unknown / missing → Png.
fn format_from_filename(filename: &str) -> OutputFormat {
    match std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some("webp") => OutputFormat::WebP,
        _ => OutputFormat::Png,
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Render the skin to an image file (headless).
    ///
    /// Format is inferred from the output filename extension
    /// (.png or .webp). Defaults to PNG.
    Render {
        /// Path to the skin PNG file.
        skin: String,

        /// Output image path. Extension determines format (.png or .webp).
        #[arg(default_value = "output.png")]
        output: String,

        #[command(flatten)]
        viewport: ViewportArgs,

        #[command(flatten)]
        scene: SceneArgs,
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
}

struct PreviewApp {
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    character: Character,
    skin: Option<eidolon::texture::Texture>,
    camera: Camera,
    texture_path: String,
    initial_size: PhysicalSize<u32>,
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Eidolon Preview")
            .with_inner_size(self.initial_size);
        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        let mut renderer =
            Renderer::new_windowed(window.clone()).expect("Failed to create windowed renderer");
        self.skin = Some(
            renderer
                .load_texture(&self.texture_path)
                .expect("Failed to load skin texture"),
        );
        let size = window.inner_size();
        renderer.resize(size.width, size.height);

        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(physical_size.width, physical_size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(skin)) = (&self.renderer, &self.skin) {
                    match renderer.render_frame(&self.character, skin, &self.camera) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                if let Some(r) = &mut self.renderer {
                                    r.resize(size.width, size.height);
                                }
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            event_loop.exit();
                        }
                        Err(e) => log::error!("Render error: {:?}", e),
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

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

    // ── format_from_filename ──

    #[test]
    fn format_from_filename_webp() {
        assert!(matches!(format_from_filename("out.webp"), OutputFormat::WebP));
        assert!(matches!(format_from_filename("/tmp/x.webp"), OutputFormat::WebP));
    }

    #[test]
    fn format_from_filename_png() {
        assert!(matches!(format_from_filename("out.png"), OutputFormat::Png));
    }

    #[test]
    fn format_from_filename_no_extension_defaults_png() {
        assert!(matches!(format_from_filename("output"), OutputFormat::Png));
        assert!(matches!(format_from_filename("/tmp/noext"), OutputFormat::Png));
        assert!(matches!(format_from_filename(""), OutputFormat::Png));
    }

    #[test]
    fn format_from_filename_unknown_extension_defaults_png() {
        assert!(matches!(format_from_filename("out.jpg"), OutputFormat::Png));
        assert!(matches!(format_from_filename("out.bmp"), OutputFormat::Png));
    }

    // ── PostureCli → Posture ──

    #[test]
    fn posture_cli_to_posture() {
        let p: Posture = PostureCli::Stand.into();
        assert_eq!(p.head_yaw, 0.0);
        assert_eq!(p.left_leg_pitch, 0.0);
    }

    #[test]
    fn posture_cli_wave_arms_raised() {
        let p: Posture = PostureCli::Wave.into();
        assert!(p.left_arm_pitch > 0.0);
    }

    #[test]
    fn posture_cli_walking_alternating() {
        let p: Posture = PostureCli::Walking.into();
        assert!(p.left_arm_pitch < 0.0);
        assert!(p.right_arm_pitch > 0.0);
        assert!(p.left_leg_pitch > 0.0);
        assert!(p.right_leg_pitch < 0.0);
    }

    #[test]
    fn posture_cli_running_larger_than_walking() {
        let w: Posture = PostureCli::Walking.into();
        let r: Posture = PostureCli::Running.into();
        assert!(r.left_arm_pitch.abs() > w.left_arm_pitch.abs());
        assert!(r.left_leg_pitch.abs() > w.left_leg_pitch.abs());
    }

    // ── character_and_camera_from_scene ──

    fn default_scene() -> SceneArgs {
        SceneArgs {
            slim: false,
            cam_yaw: 180.0,
            cam_pitch: 90.0,
            cam_zoom: 1.0,
            posture: PostureCli::Stand,
            head_yaw: None,
            head_pitch: None,
            left_arm_roll: None,
            left_arm_pitch: None,
            right_arm_roll: None,
            right_arm_pitch: None,
            left_leg_pitch: None,
            right_leg_pitch: None,
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 0.0,
            rot_x: 0.0,
            rot_y: 0.0,
            rot_z: 0.0,
        }
    }

    #[test]
    fn scene_defaults_classic_stand() {
        let (c, cam) = character_and_camera_from_scene(&default_scene());
        assert_eq!(c.skin_type, SkinType::Classic);
        assert_eq!(c.posture.head_yaw, 0.0);
        assert_eq!(c.posture.head_pitch, 0.0);
        assert_eq!(cam.yaw, 180.0);
        assert_eq!(cam.pitch, 90.0);
        assert_eq!(cam.scale, 1.0);
    }

    #[test]
    fn scene_slim_skin_type() {
        let mut scene = default_scene();
        scene.slim = true;
        let (c, _) = character_and_camera_from_scene(&scene);
        assert_eq!(c.skin_type, SkinType::Slim);
    }

    #[test]
    fn scene_camera_values_passed_through() {
        let mut scene = default_scene();
        scene.cam_yaw = 45.0;
        scene.cam_pitch = 30.0;
        scene.cam_zoom = 2.5;
        let (_, cam) = character_and_camera_from_scene(&scene);
        assert_eq!(cam.yaw, 45.0);
        assert_eq!(cam.pitch, 30.0);
        assert_eq!(cam.scale, 2.5);
    }

    #[test]
    fn scene_posture_wave_overrides_base() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Wave;
        let (c, _) = character_and_camera_from_scene(&scene);
        assert!(c.posture.left_arm_pitch > 0.0, "Wave posture: left arm raised");
        assert_eq!(c.posture.right_arm_pitch, 0.0, "Wave posture: right arm still");
    }

    #[test]
    fn scene_joint_override_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Stand;
        scene.head_yaw = Some(42.0);
        scene.left_leg_pitch = Some(-15.0);
        let (c, _) = character_and_camera_from_scene(&scene);
        assert_eq!(c.posture.head_yaw, 42.0);
        assert_eq!(c.posture.left_leg_pitch, -15.0);
        assert_eq!(c.posture.head_pitch, 0.0); // not overridden
    }

    #[test]
    fn scene_world_position_and_rotation() {
        let mut scene = default_scene();
        scene.pos_x = 1.0;
        scene.pos_y = 2.0;
        scene.pos_z = 3.0;
        scene.rot_x = 10.0;
        scene.rot_y = 20.0;
        scene.rot_z = 30.0;
        let (c, _) = character_and_camera_from_scene(&scene);
        assert_eq!(c.position.x, 1.0);
        assert_eq!(c.position.y, 2.0);
        assert_eq!(c.position.z, 3.0);
        assert_eq!(c.rotation.x, 10.0);
        assert_eq!(c.rotation.y, 20.0);
        assert_eq!(c.rotation.z, 30.0);
    }

    // ── CLI arg parsing ──

    #[test]
    fn cli_render_minimal() {
        let args = Args::try_parse_from(["eidolon", "render", "skin.png"])
            .expect("minimal render parse");
        match args.command {
            Command::Render { skin, output, viewport, scene } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(output, "output.png");
                assert_eq!(viewport.width, 800);
                assert_eq!(viewport.height, 600);
                assert_eq!(scene.cam_yaw, 180.0);
                assert!(matches!(scene.posture, PostureCli::Stand));
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn cli_render_all_options() {
        let args = Args::try_parse_from([
            "eidolon", "render", "skin.png", "out.webp",
            "--width", "400", "--height", "300",
            "--slim", "--cam-yaw", "90", "--cam-pitch", "45", "--cam-zoom", "2.0",
            "--posture", "wave",
            "--head-yaw", "15", "--left-arm-pitch", "45",
            "--pos-x", "1", "--rot-y", "180",
        ])
        .expect("full render parse");
        match args.command {
            Command::Render { skin, output, viewport, scene } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(output, "out.webp");
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
        assert!(Args::try_parse_from(["eidolon", "render", "skin.png", "--cam-zoom", "0"]).is_err());
    }

    #[test]
    fn cli_render_invalid_zoom_negative_rejected() {
        assert!(Args::try_parse_from(["eidolon", "render", "skin.png", "--cam-zoom", "-1"]).is_err());
    }

    #[test]
    fn cli_render_invalid_posture_rejected() {
        assert!(
            Args::try_parse_from(["eidolon", "render", "skin.png", "--posture", "unknown"]).is_err()
        );
    }

    #[test]
    fn cli_preview_minimal() {
        let args = Args::try_parse_from(["eidolon", "preview", "skin.png"])
            .expect("minimal preview parse");
        match args.command {
            Command::Preview { skin, viewport, scene } => {
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
            "eidolon", "preview", "skin.png",
            "--width", "1024", "--height", "768",
            "--slim", "--posture", "running", "--cam-zoom", "1.5",
        ])
        .expect("preview with options parse");
        match args.command {
            Command::Preview { skin, viewport, scene } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(viewport.width, 1024);
                assert_eq!(viewport.height, 768);
                assert!(scene.slim);
                assert!(matches!(scene.posture, PostureCli::Running));
                assert_eq!(scene.cam_zoom, 1.5);
            }
            _ => panic!("Expected Preview"),
        }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    use log::{error, info};
    match args.command {
        Command::Render {
            skin,
            output,
            viewport,
            scene,
        } => {
            // Reject output paths that attempt directory traversal.
            if std::path::Path::new(&output)
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                error!("Output path must not contain '..' components");
                return Err(Box::from(
                    "output path must not contain '..' (directory traversal)",
                ));
            }

            info!("Minecraft skin renderer");
            info!("Skin: {}", skin);
            info!("Output: {} ({}x{})", output, viewport.width, viewport.height);

            info!("Creating renderer...");
            let renderer = Renderer::new()?;
            info!("Renderer ready");

            let (character, camera) = character_and_camera_from_scene(&scene);

            info!("Loading skin: {}", skin);
            let skin_texture = renderer.load_texture(&skin)?;
            info!("Skin loaded");

            info!("Rendering...");
            let output_format = format_from_filename(&output);
            renderer.render_to_image(
                &character,
                &skin_texture,
                &camera,
                &output,
                (viewport.width, viewport.height),
                output_format,
            )?;
            info!("Done. Saved: {}", output);

            Ok(())
        }
        Command::Preview {
            skin,
            viewport,
            scene,
        } => {
            let (character, camera) = character_and_camera_from_scene(&scene);

            let event_loop = EventLoop::new()?;
            let mut app = PreviewApp {
                renderer: None,
                window: None,
                character,
                skin: None,
                camera,
                texture_path: skin,
                initial_size: PhysicalSize::new(viewport.width, viewport.height),
            };
            event_loop.run_app(&mut app)?;

            Ok(())
        }
        Command::Convert { input, output } => {
            let img =
                image::open(&input).map_err(|e| format!("Failed to open input image: {}", e))?;

            match converter::single2double(&img) {
                Ok(result) => {
                    info!("Conversion OK. Double-layer skin saved to: {:?}", output);
                    result
                        .save(output)
                        .map_err(|e| format!("Failed to save output image: {}", e))?;
                    Ok(())
                }
                Err(e) => {
                    error!("Conversion failed: {}", e);
                    Err(Box::new(std::io::Error::other(e.to_string())))
                }
            }
        }
    }
}
