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
