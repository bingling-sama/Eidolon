use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use eidolon::{
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, DefaultPostures, Posture, SkinType},
    converter,
    error::EidolonError,
    renderer::{OutputFormat, Renderer},
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
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
    Flying,
}

impl From<PostureCli> for Posture {
    fn from(value: PostureCli) -> Self {
        match value {
            PostureCli::Stand => DefaultPostures::STAND,
            PostureCli::Wave => DefaultPostures::WAVE,
            PostureCli::Walking => DefaultPostures::WALKING,
            PostureCli::Running => DefaultPostures::RUNNING,
            PostureCli::Flying => DefaultPostures::FLYING,
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

    /// Render a Minecraft-style player name above the model.
    #[arg(long)]
    nametag: Option<String>,

    /// Hide nametag pixels completely when they are behind scene geometry.
    #[arg(long)]
    disable_nametag_see_through: bool,

    /// Render a cape using the specified 64x32 PNG texture.
    #[arg(long, conflicts_with = "elytra")]
    cape: Option<String>,

    /// Render elytra using the specified 64x32 PNG texture.
    #[arg(long, conflicts_with = "cape")]
    elytra: Option<String>,

    /// Override the posture's cape pitch in degrees.
    #[arg(long)]
    cape_pitch: Option<f32>,

    /// Override the posture's elytra pitch in degrees.
    #[arg(long)]
    elytra_pitch: Option<f32>,

    /// Override the posture's elytra opening angle in degrees.
    #[arg(long)]
    elytra_spread: Option<f32>,

    /// Camera orbit yaw in degrees.
    #[arg(long, default_value_t = 180.0)]
    cam_yaw: f32,

    /// Camera orbit pitch in degrees.
    #[arg(long, default_value_t = 90.0)]
    cam_pitch: f32,

    /// Camera zoom; higher = closer (orbit radius: 4.0 / zoom).
    #[arg(long, default_value_t = 1.0, value_parser = parse_positive_scale)]
    cam_zoom: f32,

    /// Posture preset: stand, wave, walking, running, flying.
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

#[derive(Debug)]
struct SceneState {
    skin_type: SkinType,
    nametag: Option<String>,
    nametag_see_through: bool,
    back_equipment_kind: Option<BackEquipmentKind>,
    posture: Posture,
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Vector3<f32>,
}

impl SceneState {
    fn apply_to(&self, character: &mut Character) {
        character.nametag = self.nametag.clone();
        character.nametag_see_through = self.nametag_see_through;
        character.posture = self.posture;
        character.position = self.position;
        character.rotation = self.rotation;
    }
}

fn scene_state_and_camera_from_scene(scene: &SceneArgs) -> (SceneState, Camera) {
    let base: Posture = scene.posture.into();
    let posture = Posture {
        cape_pitch: scene.cape_pitch.unwrap_or(base.cape_pitch),
        body_pitch: base.body_pitch,
        elytra_pitch: scene.elytra_pitch.unwrap_or(base.elytra_pitch),
        elytra_yaw: base.elytra_yaw,
        elytra_roll: scene.elytra_spread.unwrap_or(base.elytra_roll),
        head_yaw: scene.head_yaw.unwrap_or(base.head_yaw),
        head_pitch: scene.head_pitch.unwrap_or(base.head_pitch),
        left_arm_roll: scene.left_arm_roll.unwrap_or(base.left_arm_roll),
        left_arm_pitch: scene.left_arm_pitch.unwrap_or(base.left_arm_pitch),
        right_arm_roll: scene.right_arm_roll.unwrap_or(base.right_arm_roll),
        right_arm_pitch: scene.right_arm_pitch.unwrap_or(base.right_arm_pitch),
        left_leg_pitch: scene.left_leg_pitch.unwrap_or(base.left_leg_pitch),
        right_leg_pitch: scene.right_leg_pitch.unwrap_or(base.right_leg_pitch),
    };
    let state = SceneState {
        skin_type: if scene.slim {
            SkinType::Slim
        } else {
            SkinType::Classic
        },
        nametag: scene.nametag.clone(),
        nametag_see_through: !scene.disable_nametag_see_through,
        back_equipment_kind: scene
            .cape
            .as_ref()
            .map(|_| BackEquipmentKind::Cape)
            .or_else(|| scene.elytra.as_ref().map(|_| BackEquipmentKind::Elytra)),
        posture,
        position: cgmath::Vector3::new(scene.pos_x, scene.pos_y, scene.pos_z),
        rotation: cgmath::Vector3::new(scene.rot_x, scene.rot_y, scene.rot_z),
    };

    let camera = Camera {
        yaw: scene.cam_yaw,
        pitch: scene.cam_pitch,
        scale: scene.cam_zoom,
    };
    (state, camera)
}

fn load_character(
    renderer: &Renderer,
    skin_path: &str,
    scene: &SceneArgs,
) -> Result<(Character, Camera), EidolonError> {
    let (state, camera) = scene_state_and_camera_from_scene(scene);
    let body = renderer.load_body(skin_path, state.skin_type)?;
    let mut character = Character::from_body(body);
    state.apply_to(&mut character);

    let atlas_path = scene.cape.as_deref().or(scene.elytra.as_deref());
    if let (Some(path), Some(kind)) = (atlas_path, state.back_equipment_kind) {
        let atlas = renderer.load_back_equipment_atlas(path)?;
        character.equip(BackEquipment::new(atlas, kind));
    }

    Ok((character, camera))
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
    character: Option<Character>,
    scene_state: SceneState,
    camera: Camera,
    texture_path: String,
    cape_texture_path: Option<String>,
    elytra_texture_path: Option<String>,
    initial_size: PhysicalSize<u32>,
    interaction: PreviewInteraction,
}

#[derive(Default)]
struct PreviewInteraction {
    left_pressed: bool,
    right_pressed: bool,
    last_cursor: Option<(f64, f64)>,
    pan_x: f32,
    pan_y: f32,
    base_position: Option<cgmath::Vector3<f32>>,
}

impl PreviewInteraction {
    fn mouse_input(&mut self, button: MouseButton, pressed: bool) {
        if button == MouseButton::Left {
            self.left_pressed = pressed;
        } else if button == MouseButton::Right {
            self.right_pressed = pressed;
        }
    }

    fn cursor_moved(
        &mut self,
        x: f64,
        y: f64,
        position: &mut cgmath::Vector3<f32>,
        camera: &mut Camera,
    ) {
        self.base_position.get_or_insert(*position);
        if let Some((last_x, last_y)) = self.last_cursor {
            let dx = (x - last_x) as f32;
            let dy = (y - last_y) as f32;
            if self.left_pressed {
                self.pan_x += dx;
                self.pan_y -= dy;
            }
            if self.right_pressed {
                camera.yaw -= dx * 0.4;
                camera.pitch = (camera.pitch + dy * 0.4).clamp(1.0, 179.0);
            }
            self.update_pan(position, camera);
        }
        self.last_cursor = Some((x, y));
    }

    fn scroll(&mut self, delta: f32, position: &mut cgmath::Vector3<f32>, camera: &mut Camera) {
        self.base_position.get_or_insert(*position);
        camera.scale = (camera.scale * 1.1_f32.powf(delta)).clamp(0.1, 10.0);
        self.update_pan(position, camera);
    }

    fn update_pan(&self, position: &mut cgmath::Vector3<f32>, camera: &Camera) {
        let Some(base) = self.base_position else {
            return;
        };
        let yaw = camera.yaw.to_radians();
        let pitch = (camera.pitch - 90.0).to_radians();
        let right = cgmath::Vector3::new(yaw.cos(), 0.0, -yaw.sin());
        let up = cgmath::Vector3::new(
            -yaw.sin() * pitch.sin(),
            pitch.cos(),
            -yaw.cos() * pitch.sin(),
        );
        let distance_per_pixel = 0.005 / camera.scale;
        *position = base + (right * self.pan_x + up * self.pan_y) * distance_per_pixel;
    }
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
        let body = renderer
            .load_body(&self.texture_path, self.scene_state.skin_type)
            .expect("Failed to load body");
        let mut character = Character::from_body(body);
        self.scene_state.apply_to(&mut character);
        let atlas_path = self
            .cape_texture_path
            .as_deref()
            .or(self.elytra_texture_path.as_deref());
        if let (Some(path), Some(kind)) = (atlas_path, self.scene_state.back_equipment_kind) {
            let atlas = renderer
                .load_back_equipment_atlas(path)
                .expect("Failed to load back equipment atlas");
            character.equip(BackEquipment::new(atlas, kind));
        }
        let size = window.inner_size();
        renderer.resize(size.width, size.height);

        self.renderer = Some(renderer);
        self.character = Some(character);
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
            WindowEvent::MouseInput { state, button, .. } => {
                self.interaction
                    .mouse_input(button, state == ElementState::Pressed);
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(character) = &mut self.character {
                    self.interaction.cursor_moved(
                        position.x,
                        position.y,
                        &mut character.position,
                        &mut self.camera,
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 50.0,
                };
                if let Some(character) = &mut self.character {
                    self.interaction
                        .scroll(lines, &mut character.position, &mut self.camera);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(character)) = (&self.renderer, &self.character) {
                    match renderer.render_frame(character, &self.camera) {
                        Ok(()) => {}
                        Err(EidolonError::Surface(wgpu::SurfaceError::Lost)) => {
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                if let Some(r) = &mut self.renderer {
                                    r.resize(size.width, size.height);
                                }
                            }
                        }
                        Err(EidolonError::Surface(wgpu::SurfaceError::OutOfMemory)) => {
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

    #[test]
    fn preview_left_drag_pans_without_rotating() {
        let mut interaction = PreviewInteraction::default();
        let mut position = cgmath::Vector3::new(0.0, 0.0, 0.0);
        let mut camera = Camera::default();

        interaction.mouse_input(winit::event::MouseButton::Left, true);
        interaction.cursor_moved(100.0, 100.0, &mut position, &mut camera);
        interaction.cursor_moved(120.0, 110.0, &mut position, &mut camera);

        assert_ne!(position, cgmath::Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(camera.yaw, 180.0);
        assert_eq!(camera.pitch, 90.0);
    }

    #[test]
    fn preview_right_drag_rotates_in_pointer_direction_and_clamps_pitch() {
        let mut interaction = PreviewInteraction::default();
        let mut position = cgmath::Vector3::new(0.0, 0.0, 0.0);
        let mut camera = Camera::default();

        interaction.mouse_input(winit::event::MouseButton::Right, true);
        interaction.cursor_moved(100.0, 100.0, &mut position, &mut camera);
        interaction.cursor_moved(120.0, -1000.0, &mut position, &mut camera);

        assert_eq!(camera.yaw, 172.0);
        assert_eq!(camera.pitch, 1.0);
        assert_eq!(position, cgmath::Vector3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn preview_scroll_zooms_with_safe_limits() {
        let mut interaction = PreviewInteraction::default();
        let mut position = cgmath::Vector3::new(0.0, 0.0, 0.0);
        let mut camera = Camera::default();

        interaction.scroll(1.0, &mut position, &mut camera);
        assert!(camera.scale > 1.0);

        interaction.scroll(-1000.0, &mut position, &mut camera);
        assert_eq!(camera.scale, 0.1);

        interaction.scroll(1000.0, &mut position, &mut camera);
        assert_eq!(camera.scale, 10.0);
    }

    #[test]
    fn preview_pan_stays_in_view_plane_after_rotation() {
        let mut interaction = PreviewInteraction::default();
        let mut position = cgmath::Vector3::new(0.0, 0.0, 0.0);
        let mut camera = Camera::default();

        interaction.mouse_input(MouseButton::Left, true);
        interaction.cursor_moved(0.0, 0.0, &mut position, &mut camera);
        interaction.cursor_moved(20.0, 10.0, &mut position, &mut camera);
        interaction.mouse_input(MouseButton::Left, false);
        let before = position;

        interaction.mouse_input(MouseButton::Right, true);
        interaction.cursor_moved(220.0, -40.0, &mut position, &mut camera);

        assert_ne!(position, before);
        assert_ne!(position.z, 0.0);
    }

    #[test]
    fn preview_pan_offset_scales_with_zoom() {
        let mut interaction = PreviewInteraction::default();
        let mut position = cgmath::Vector3::new(0.0, 0.0, 0.0);
        let mut camera = Camera::default();

        interaction.mouse_input(MouseButton::Left, true);
        interaction.cursor_moved(0.0, 0.0, &mut position, &mut camera);
        interaction.cursor_moved(20.0, 0.0, &mut position, &mut camera);
        let before = position.x.abs();

        interaction.scroll(1.0, &mut position, &mut camera);

        assert!(position.x.abs() < before);
    }

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
        assert!(matches!(
            format_from_filename("out.webp"),
            OutputFormat::WebP
        ));
        assert!(matches!(
            format_from_filename("/tmp/x.webp"),
            OutputFormat::WebP
        ));
    }

    #[test]
    fn format_from_filename_png() {
        assert!(matches!(format_from_filename("out.png"), OutputFormat::Png));
    }

    #[test]
    fn format_from_filename_no_extension_defaults_png() {
        assert!(matches!(format_from_filename("output"), OutputFormat::Png));
        assert!(matches!(
            format_from_filename("/tmp/noext"),
            OutputFormat::Png
        ));
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

    #[test]
    fn posture_cli_flying_spreads_elytra() {
        let posture: Posture = PostureCli::Flying.into();

        assert_eq!(posture.elytra_pitch, 20.0);
        assert_eq!(posture.elytra_yaw, 0.0);
        assert_eq!(posture.elytra_roll, 60.0);
        assert_eq!(posture.body_pitch, -90.0);
        assert_eq!(posture.head_pitch, 45.0);
    }

    // ── scene_state_and_camera_from_scene ──

    fn default_scene() -> SceneArgs {
        SceneArgs {
            slim: false,
            nametag: None,
            disable_nametag_see_through: false,
            cape: None,
            elytra: None,
            cape_pitch: None,
            elytra_pitch: None,
            elytra_spread: None,
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
        let (state, cam) = scene_state_and_camera_from_scene(&default_scene());
        assert_eq!(state.skin_type, SkinType::Classic);
        assert_eq!(state.posture.head_yaw, 0.0);
        assert_eq!(state.posture.head_pitch, 0.0);
        assert_eq!(cam.yaw, 180.0);
        assert_eq!(cam.pitch, 90.0);
        assert_eq!(cam.scale, 1.0);
    }

    #[test]
    fn scene_slim_skin_type() {
        let mut scene = default_scene();
        scene.slim = true;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.skin_type, SkinType::Slim);
    }

    #[test]
    fn scene_nametag_is_passed_to_character() {
        let mut scene = default_scene();
        scene.nametag = Some("Steve".to_owned());
        let (state, _) = scene_state_and_camera_from_scene(&scene);

        assert_eq!(state.nametag.as_deref(), Some("Steve"));
    }

    #[test]
    fn scene_can_disable_nametag_see_through() {
        let default_state = scene_state_and_camera_from_scene(&default_scene()).0;
        assert!(default_state.nametag_see_through);

        let mut scene = default_scene();
        scene.disable_nametag_see_through = true;
        let state = scene_state_and_camera_from_scene(&scene).0;
        assert!(!state.nametag_see_through);
    }

    #[test]
    fn scene_cape_path_enables_cape_rendering() {
        let mut scene = default_scene();
        scene.cape = Some("cape.png".to_owned());
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.back_equipment_kind, Some(BackEquipmentKind::Cape));
    }

    #[test]
    fn scene_elytra_path_enables_elytra_rendering() {
        let mut scene = default_scene();
        scene.elytra = Some("elytra.png".to_owned());
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.back_equipment_kind, Some(BackEquipmentKind::Elytra));
    }

    #[test]
    fn scene_camera_values_passed_through() {
        let mut scene = default_scene();
        scene.cam_yaw = 45.0;
        scene.cam_pitch = 30.0;
        scene.cam_zoom = 2.5;
        let (_, cam) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(cam.yaw, 45.0);
        assert_eq!(cam.pitch, 30.0);
        assert_eq!(cam.scale, 2.5);
    }

    #[test]
    fn scene_posture_wave_overrides_base() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Wave;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert!(
            state.posture.left_arm_pitch > 0.0,
            "Wave posture: left arm raised"
        );
        assert_eq!(
            state.posture.right_arm_pitch, 0.0,
            "Wave posture: right arm still"
        );
        assert_eq!(state.posture.cape_pitch, 10.8);
    }

    #[test]
    fn scene_running_uses_running_cape_pitch() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Running;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.cape_pitch, 54.0);
    }

    #[test]
    fn scene_cape_pitch_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Running;
        scene.cape_pitch = Some(25.0);
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.cape_pitch, 25.0);
    }

    #[test]
    fn scene_joint_override_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Stand;
        scene.head_yaw = Some(42.0);
        scene.left_leg_pitch = Some(-15.0);
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.head_yaw, 42.0);
        assert_eq!(state.posture.left_leg_pitch, -15.0);
        assert_eq!(state.posture.head_pitch, 0.0); // not overridden
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
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.position.x, 1.0);
        assert_eq!(state.position.y, 2.0);
        assert_eq!(state.position.z, 3.0);
        assert_eq!(state.rotation.x, 10.0);
        assert_eq!(state.rotation.y, 20.0);
        assert_eq!(state.rotation.z, 30.0);
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
            } => {
                assert_eq!(skin, "skin.png");
                assert_eq!(output, "output.png");
                assert_eq!(viewport.width, 800);
                assert_eq!(viewport.height, 600);
                assert_eq!(scene.cam_yaw, 180.0);
                assert!(matches!(scene.posture, PostureCli::Stand));
                assert_eq!(scene.nametag, None);
                assert_eq!(scene.cape, None);
                assert_eq!(scene.elytra, None);
            }
            _ => panic!("Expected Render"),
        }
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
                "flying",
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

            let (state, _) = scene_state_and_camera_from_scene(&scene);
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
            } => {
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
    fn cli_preview_minimal() {
        let args = Args::try_parse_from(["eidolon", "preview", "skin.png"])
            .expect("minimal preview parse");
        match args.command {
            Command::Preview {
                skin,
                viewport,
                scene,
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
            "running",
            "--cam-zoom",
            "1.5",
        ])
        .expect("preview with options parse");
        match args.command {
            Command::Preview {
                skin,
                viewport,
                scene,
            } => {
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
            info!(
                "Output: {} ({}x{})",
                output, viewport.width, viewport.height
            );

            info!("Creating renderer...");
            let renderer = Renderer::new()?;
            info!("Renderer ready");

            info!("Loading character resources");
            let (character, camera) = load_character(&renderer, &skin, &scene)?;
            info!("Character resources loaded");

            info!("Rendering...");
            let output_format = format_from_filename(&output);
            renderer.render_to_image(
                &character,
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
            let (scene_state, camera) = scene_state_and_camera_from_scene(&scene);

            let event_loop = EventLoop::new()?;
            let mut app = PreviewApp {
                renderer: None,
                window: None,
                character: None,
                scene_state,
                camera,
                texture_path: skin,
                cape_texture_path: scene.cape,
                elytra_texture_path: scene.elytra,
                initial_size: PhysicalSize::new(viewport.width, viewport.height),
                interaction: PreviewInteraction::default(),
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
