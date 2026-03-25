use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use eidolon::{
    camera::Camera,
    character::{Character, DefaultPostures, Posture, SkinType},
    renderer::{OutputFormat, Renderer},
    utils::converter,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Minecraft skin renderer and skin-atlas utilities.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
enum SkinTypeCli {
    Classic,
    Slim,
}

impl From<SkinTypeCli> for SkinType {
    fn from(value: SkinTypeCli) -> Self {
        match value {
            SkinTypeCli::Classic => SkinType::Classic,
            SkinTypeCli::Slim => SkinType::Slim,
        }
    }
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

#[derive(Parser, Debug)]
struct SceneArgs {
    /// Path to the skin PNG (decoded as RGBA; single-layer skins are expanded when loaded).
    #[arg(long, default_value = "resources/bingling_sama.png")]
    texture: String,

    /// Arm geometry: `classic` (wide) or `slim`.
    #[arg(long, value_enum)]
    skin_type: SkinTypeCli,

    /// Camera orbit yaw in degrees (`Camera::yaw`).
    #[arg(long, default_value_t = 180.0)]
    yaw: f32,

    /// Camera orbit pitch in degrees (`Camera::pitch`).
    #[arg(long, default_value_t = 90.0)]
    pitch: f32,

    /// Camera distance scale; must be > 0 (smaller orbit radius when larger).
    #[arg(long, default_value_t = 1.0, value_parser = parse_positive_scale)]
    scale: f32,

    /// Posture: `stand` (default).
    #[arg(long, value_enum, default_value_t = PostureCli::Stand)]
    posture: PostureCli,

    /// Override head yaw in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    head_yaw: Option<f32>,
    /// Override head pitch in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    head_pitch: Option<f32>,
    /// Override left arm roll in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    left_arm_roll: Option<f32>,
    /// Override left arm pitch in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    left_arm_pitch: Option<f32>,
    /// Override right arm roll in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    right_arm_roll: Option<f32>,
    /// Override right arm pitch in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    right_arm_pitch: Option<f32>,
    /// Override left leg pitch in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    left_leg_pitch: Option<f32>,
    /// Override right leg pitch in degrees; if omitted, uses the `--posture` preset.
    #[arg(long)]
    right_leg_pitch: Option<f32>,

    /// Character world position X.
    #[arg(long, default_value_t = 0.0)]
    position_x: f32,
    /// Character world position Y.
    #[arg(long, default_value_t = 0.0)]
    position_y: f32,
    /// Character world position Z.
    #[arg(long, default_value_t = 0.0)]
    position_z: f32,

    /// Character rotation about X in degrees (Euler order X → Y → Z in uniforms).
    #[arg(long, default_value_t = 0.0)]
    rotation_x: f32,
    /// Character rotation about Y in degrees.
    #[arg(long, default_value_t = 0.0)]
    rotation_y: f32,
    /// Character rotation about Z in degrees.
    #[arg(long, default_value_t = 0.0)]
    rotation_z: f32,
}

fn character_and_camera_from_scene(scene: &SceneArgs) -> (Character, Camera) {
    let mut character = Character::new();
    character.skin_type = scene.skin_type.into();
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
    character.position = cgmath::Vector3::new(scene.position_x, scene.position_y, scene.position_z);
    character.rotation = cgmath::Vector3::new(scene.rotation_x, scene.rotation_y, scene.rotation_z);

    let camera = Camera {
        yaw: scene.yaw,
        pitch: scene.pitch,
        scale: scene.scale,
    };
    (character, camera)
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Render the skin to an image file (headless).
    Render {
        /// Output file path.
        #[arg(long, default_value = "output.png")]
        filename: String,

        /// Image format: `png` or `webp`.
        #[arg(long, default_value = "png")]
        format: String,

        #[command(flatten)]
        viewport: ViewportArgs,

        #[command(flatten)]
        scene: SceneArgs,
    },
    /// Open a live preview window.
    Preview {
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

        let mut renderer = Renderer::new_windowed(window.clone());
        self.character.skin = Some(renderer.load_texture(&self.texture_path).unwrap());
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
                if let Some(renderer) = &self.renderer {
                    match renderer.render_frame(&self.character, &self.camera) {
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
            filename,
            format,
            viewport,
            scene,
        } => {
            info!("Minecraft skin renderer");
            info!("Output file: {}", filename);
            info!("Size: {}x{}", viewport.width, viewport.height);
            info!("Skin: {}", scene.texture);

            info!("Creating renderer...");
            let renderer = Renderer::new();
            info!("Renderer ready");

            let (mut character, camera) = character_and_camera_from_scene(&scene);

            info!("Loading skin: {}", scene.texture);
            character.skin = Some(renderer.load_texture(&scene.texture)?);
            info!("Skin loaded");

            info!("Rendering...");

            let output_format = match format.to_lowercase().as_str() {
                "png" => OutputFormat::Png,
                "webp" => OutputFormat::WebP,
                other => {
                    error!("Unsupported output format: {} (use png or webp)", other);
                    return Err(Box::from("unsupported output format"));
                }
            };

            let mut filename = filename;
            if filename == "output.png" {
                filename = match format.to_lowercase().as_str() {
                    "png" => "output.png".to_string(),
                    "webp" => "output.webp".to_string(),
                    _ => filename,
                };
            }

            renderer.render_to_image(
                &character,
                &camera,
                &filename,
                (viewport.width, viewport.height),
                output_format,
            )?;
            info!("Done. Saved: {}", filename);

            Ok(())
        }
        Command::Preview { viewport, scene } => {
            let (character, camera) = character_and_camera_from_scene(&scene);

            let event_loop = EventLoop::new()?;
            let mut app = PreviewApp {
                renderer: None,
                window: None,
                character,
                camera,
                texture_path: scene.texture,
                initial_size: PhysicalSize::new(viewport.width, viewport.height),
            };
            event_loop.run_app(&mut app)?;

            Ok(())
        }
        Command::Convert { input, output } => {
            let img =
                image::open(input).map_err(|e| format!("Failed to open input image: {}", e))?;

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
                    Err(Box::new(std::io::Error::other(e)))
                }
            }
        }
    }
}
