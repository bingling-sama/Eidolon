use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use eidolon_core::{
    animation::{AnimationPreset, AnimationTimeline},
    assets::RawSkin,
    back_equipment::{BackEquipment, BackEquipmentKind},
    camera::Camera,
    character::{Character, Posture, SkinType},
    error::EidolonError,
    output::{write_output, OutputIntent},
    renderer::Renderer,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

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
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq, Eq)]
enum PostureCli {
    Idle,
    Walk,
    Run,
    Wave,
    Fly,
}

impl PostureCli {
    #[cfg(test)]
    const ALL: [Self; 5] = [Self::Idle, Self::Walk, Self::Run, Self::Wave, Self::Fly];

    const fn animation_preset(self) -> AnimationPreset {
        match self {
            PostureCli::Idle => AnimationPreset::Idle,
            PostureCli::Walk => AnimationPreset::Walk,
            PostureCli::Run => AnimationPreset::Run,
            PostureCli::Wave => AnimationPreset::Wave,
            PostureCli::Fly => AnimationPreset::Fly,
        }
    }

    /// Phase used for static rendering and the initial paused preview.
    fn sample_time(self) -> std::time::Duration {
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

fn parse_animation_speed(s: &str) -> Result<f32, String> {
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
struct ViewportArgs {
    /// Output image or window width in pixels.
    #[arg(long, default_value_t = 800, value_parser = clap::value_parser!(u32).range(1..))]
    width: u32,
    /// Output image or window height in pixels.
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u32).range(1..))]
    height: u32,
}

#[derive(Parser, Debug)]
struct RenderAnimationArgs {
    /// Render a complete animation cycle instead of a static frame.
    #[arg(long)]
    animate: bool,

    /// Animation frames per second.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=60), requires = "animate")]
    fps: u32,

    /// Animation playback speed multiplier.
    #[arg(long, default_value_t = 1.0, value_parser = parse_animation_speed, requires = "animate")]
    speed: f32,

    /// Animation repetitions in the output metadata; 0 loops forever.
    #[arg(long, default_value_t = 0, requires = "animate")]
    loop_count: u16,
}

fn resolve_render_output(
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

    /// Action posture: idle, walk, run, wave, fly.
    #[arg(long, value_enum, default_value_t = PostureCli::Idle)]
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

#[derive(Clone, Copy, Default)]
struct PostureOverrides {
    cape_pitch: Option<f32>,
    elytra_pitch: Option<f32>,
    elytra_spread: Option<f32>,
    head_yaw: Option<f32>,
    head_pitch: Option<f32>,
    left_arm_roll: Option<f32>,
    left_arm_pitch: Option<f32>,
    right_arm_roll: Option<f32>,
    right_arm_pitch: Option<f32>,
    left_leg_pitch: Option<f32>,
    right_leg_pitch: Option<f32>,
}

impl From<&SceneArgs> for PostureOverrides {
    fn from(scene: &SceneArgs) -> Self {
        Self {
            cape_pitch: scene.cape_pitch,
            elytra_pitch: scene.elytra_pitch,
            elytra_spread: scene.elytra_spread,
            head_yaw: scene.head_yaw,
            head_pitch: scene.head_pitch,
            left_arm_roll: scene.left_arm_roll,
            left_arm_pitch: scene.left_arm_pitch,
            right_arm_roll: scene.right_arm_roll,
            right_arm_pitch: scene.right_arm_pitch,
            left_leg_pitch: scene.left_leg_pitch,
            right_leg_pitch: scene.right_leg_pitch,
        }
    }
}

impl PostureOverrides {
    fn apply(self, base: Posture) -> Posture {
        Posture {
            cape_pitch: self.cape_pitch.unwrap_or(base.cape_pitch),
            body_pitch: base.body_pitch,
            elytra_pitch: self.elytra_pitch.unwrap_or(base.elytra_pitch),
            elytra_yaw: base.elytra_yaw,
            elytra_roll: self.elytra_spread.unwrap_or(base.elytra_roll),
            head_yaw: self.head_yaw.unwrap_or(base.head_yaw),
            head_pitch: self.head_pitch.unwrap_or(base.head_pitch),
            left_arm_roll: self.left_arm_roll.unwrap_or(base.left_arm_roll),
            left_arm_pitch: self.left_arm_pitch.unwrap_or(base.left_arm_pitch),
            right_arm_roll: self.right_arm_roll.unwrap_or(base.right_arm_roll),
            right_arm_pitch: self.right_arm_pitch.unwrap_or(base.right_arm_pitch),
            left_leg_pitch: self.left_leg_pitch.unwrap_or(base.left_leg_pitch),
            right_leg_pitch: self.right_leg_pitch.unwrap_or(base.right_leg_pitch),
        }
    }
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

fn apply_posture_overrides(base: Posture, scene: &SceneArgs) -> Posture {
    PostureOverrides::from(scene).apply(base)
}

fn scene_state_and_camera_from_scene(scene: &SceneArgs) -> (SceneState, Camera) {
    let base: Posture = scene.posture.into();
    let posture = apply_posture_overrides(base, scene);
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

#[derive(Subcommand, Debug)]
enum Command {
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

struct PreviewApp {
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    character: Option<Character>,
    scene_state: SceneState,
    camera: Camera,
    texture_path: String,
    cape_atlas_path: Option<String>,
    elytra_atlas_path: Option<String>,
    initial_size: PhysicalSize<u32>,
    interaction: PreviewInteraction,
    playback: PreviewPlayback,
    posture_overrides: PostureOverrides,
}

struct PreviewPlayback {
    timeline: AnimationTimeline,
    last_tick: Option<std::time::Instant>,
}

impl PreviewPlayback {
    fn new(preset: AnimationPreset, playing: bool) -> Self {
        let mut timeline = AnimationTimeline::new(preset);
        if playing {
            timeline.play();
        }
        Self {
            timeline,
            last_tick: None,
        }
    }

    fn from_posture(posture: PostureCli) -> Self {
        let mut playback = Self::new(posture.animation_preset(), false);
        playback.timeline.seek_to(posture.sample_time());
        playback
    }

    fn tick(&mut self, now: std::time::Instant) {
        if let Some(previous) = self.last_tick {
            let delta = now.saturating_duration_since(previous);
            self.timeline.advance(delta);
        }
        self.last_tick = Some(now);
    }

    fn toggle(&mut self, now: std::time::Instant) {
        self.tick(now);
        if self.timeline.is_playing() {
            self.timeline.pause();
        } else {
            self.timeline.play();
        }
    }

    fn suspend(&mut self) {
        self.last_tick = None;
    }

    fn select(&mut self, preset: AnimationPreset, now: std::time::Instant) {
        self.timeline.set_preset(preset);
        self.last_tick = Some(now);
    }

    fn seek_frames(&mut self, frames: i32) {
        let delta = std::time::Duration::from_secs_f64(f64::from(frames.unsigned_abs()) / 30.0);
        if frames.is_negative() {
            self.timeline.seek_back(delta);
        } else {
            self.timeline.seek_by(delta);
        }
    }

    fn set_speed(&mut self, speed: f32) {
        self.timeline.set_speed(speed);
    }

    fn adjust_speed(&mut self, factor: f32) {
        self.set_speed(self.timeline.speed() * factor);
    }

    #[cfg(test)]
    fn elapsed(&self) -> std::time::Duration {
        self.timeline.position()
    }

    fn sample(&self) -> Posture {
        self.timeline.sample()
    }
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
            .cape_atlas_path
            .as_deref()
            .or(self.elytra_atlas_path.as_deref());
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
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                let now = std::time::Instant::now();
                match event.logical_key {
                    Key::Named(NamedKey::Space) => self.playback.toggle(now),
                    Key::Named(NamedKey::ArrowLeft) => self.playback.seek_frames(-1),
                    Key::Named(NamedKey::ArrowRight) => self.playback.seek_frames(1),
                    Key::Character(ref value) => match value.as_ref() {
                        "1" => self.playback.select(AnimationPreset::Idle, now),
                        "2" => self.playback.select(AnimationPreset::Walk, now),
                        "3" => self.playback.select(AnimationPreset::Run, now),
                        "4" => self.playback.select(AnimationPreset::Wave, now),
                        "5" => self.playback.select(AnimationPreset::Fly, now),
                        "-" => self.playback.adjust_speed(0.8),
                        "+" | "=" => self.playback.adjust_speed(1.25),
                        _ => {}
                    },
                    _ => {}
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(_) => self.playback.suspend(),
            WindowEvent::RedrawRequested => {
                self.playback.tick(std::time::Instant::now());
                if let Some(character) = &mut self.character {
                    character.posture = self.posture_overrides.apply(self.playback.sample());
                }
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

    #[test]
    fn preview_playback_pauses_seeks_scales_time_and_ignores_long_gaps() {
        let start = std::time::Instant::now();
        let mut playback = PreviewPlayback::new(AnimationPreset::Walk, true);

        playback.tick(start);
        playback.tick(start + std::time::Duration::from_millis(100));
        assert_eq!(playback.elapsed(), std::time::Duration::from_millis(100));

        playback.toggle(start + std::time::Duration::from_millis(100));
        playback.tick(start + std::time::Duration::from_millis(200));
        assert_eq!(playback.elapsed(), std::time::Duration::from_millis(100));
        playback.seek_frames(1);
        assert_eq!(
            playback.elapsed(),
            std::time::Duration::from_millis(100) + std::time::Duration::from_secs_f64(1.0 / 30.0)
        );

        playback.toggle(start + std::time::Duration::from_millis(200));
        playback.set_speed(2.0);
        playback.tick(start + std::time::Duration::from_millis(250));
        assert_eq!(
            playback.elapsed(),
            std::time::Duration::from_millis(200) + std::time::Duration::from_secs_f64(1.0 / 30.0)
        );
        playback.tick(start + std::time::Duration::from_secs(2));
        assert_eq!(
            playback.elapsed(),
            std::time::Duration::from_millis(200) + std::time::Duration::from_secs_f64(1.0 / 30.0)
        );
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
            posture: PostureCli::Idle,
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
    fn scene_defaults_to_classic_idle() {
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
        assert_eq!(state.posture, PostureCli::Wave.into());
    }

    #[test]
    fn scene_run_uses_representative_frame() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Run;
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture, PostureCli::Run.into());
    }

    #[test]
    fn scene_cape_pitch_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Run;
        scene.cape_pitch = Some(25.0);
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.cape_pitch, 25.0);
    }

    #[test]
    fn scene_joint_override_overrides_posture() {
        let mut scene = default_scene();
        scene.posture = PostureCli::Idle;
        scene.head_yaw = Some(42.0);
        scene.left_leg_pitch = Some(-15.0);
        let (state, _) = scene_state_and_camera_from_scene(&scene);
        assert_eq!(state.posture.head_yaw, 42.0);
        assert_eq!(state.posture.left_leg_pitch, -15.0);
        assert_eq!(state.posture.head_pitch, 0.0); // not overridden
    }

    #[test]
    fn animation_sample_is_modified_only_by_explicit_overrides() {
        let mut scene = default_scene();
        scene.head_yaw = Some(42.0);
        scene.elytra_spread = Some(33.0);
        let sampled = AnimationPreset::Walk.sample(AnimationPreset::Walk.cycle_duration() / 4);

        let posture = apply_posture_overrides(sampled, &scene);

        assert_eq!(posture.head_yaw, 42.0);
        assert_eq!(posture.elytra_roll, 33.0);
        assert_eq!(posture.left_leg_pitch, sampled.left_leg_pitch);
        assert_eq!(posture.right_arm_pitch, sampled.right_arm_pitch);
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
    fn preview_posture_selects_the_animation_used_when_playback_starts() {
        let args = Args::try_parse_from(["eidolon", "preview", "skin.png", "--posture", "fly"])
            .expect("flying preview should parse");

        let Command::Preview { scene, .. } = args.command else {
            panic!("Expected Preview");
        };
        let static_posture: Posture = scene.posture.into();
        let mut playback = PreviewPlayback::from_posture(scene.posture);

        assert_eq!(scene.posture, PostureCli::Fly);
        assert_eq!(playback.timeline.preset(), AnimationPreset::Fly);
        assert!(!playback.timeline.is_playing());
        assert_eq!(playback.sample(), static_posture);

        playback.toggle(std::time::Instant::now());
        assert_eq!(playback.sample(), static_posture);
    }

    #[test]
    fn preview_starts_every_animation_from_its_static_representative_frame() {
        for posture in PostureCli::ALL {
            let static_posture: Posture = posture.into();
            let mut playback = PreviewPlayback::from_posture(posture);

            playback.toggle(std::time::Instant::now());

            assert_eq!(playback.sample(), static_posture, "{posture:?}");
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
            animation,
        } => {
            let output = match resolve_render_output(output, &viewport, &animation) {
                Ok(resolved) => resolved,
                Err(message) => {
                    error!("{message}");
                    return Err(message.into());
                }
            };

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

            let output_path = PathBuf::from(&output);
            let output_intent = if animation.animate {
                OutputIntent::animation_from_path(
                    &output_path,
                    scene.posture.animation_preset(),
                    animation.fps,
                    animation.speed,
                    animation.loop_count,
                    (viewport.width, viewport.height),
                )?
            } else {
                OutputIntent::static_from_path(&output_path, (viewport.width, viewport.height))
            };

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
            let (mut character, camera) = load_character(&renderer, &skin, &scene)?;
            info!("Character resources loaded");

            info!("Rendering...");
            let mut frame_producer = |time| {
                if animation.animate {
                    let sampled = scene.posture.animation_preset().sample(time);
                    character.posture = apply_posture_overrides(sampled, &scene);
                }
                renderer.render(&character, &camera, viewport.width, viewport.height)
            };
            let written = write_output(&output_path, output_intent, &mut frame_producer)?;
            info!("Done. Saved: {}", written.display());
            Ok(())
        }
        Command::Preview {
            skin,
            viewport,
            scene,
        } => {
            let (scene_state, camera) = scene_state_and_camera_from_scene(&scene);
            let posture_overrides = PostureOverrides::from(&scene);
            let playback = PreviewPlayback::from_posture(scene.posture);

            let event_loop = EventLoop::new()?;
            let mut app = PreviewApp {
                renderer: None,
                window: None,
                character: None,
                scene_state,
                camera,
                texture_path: skin,
                cape_atlas_path: scene.cape,
                elytra_atlas_path: scene.elytra,
                initial_size: PhysicalSize::new(viewport.width, viewport.height),
                interaction: PreviewInteraction::default(),
                playback,
                posture_overrides,
            };
            event_loop.run_app(&mut app)?;

            Ok(())
        }
        Command::Convert { input, output } => {
            match RawSkin::convert_file(&input, &output) {
                Ok(()) => {
                    info!("Conversion OK. Double-layer skin saved to: {:?}", output);
                    Ok(())
                }
                Err(e) => {
                    error!("Conversion failed: {}", e);
                    Err(Box::new(std::io::Error::other(e.to_string())))
                }
            }
        }
        Command::Identify { texture, json } => {
            let bytes = std::fs::read(&texture).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("failed to read '{}': {}", texture.display(), e),
                )) as Box<dyn std::error::Error>
            })?;
            let report = eidolon_core::classifier::inspect_texture(&bytes).map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
            })?;

            if json {
                println!(
                    "{{\n  \"kind\": \"{}\",\n  \"width\": {},\n  \"height\": {},\n  \"hd_scale\": {},\n  \"is_hd\": {},\n  \"can_auto_upgrade\": {},\n  \"description\": \"{}\"\n}}",
                    report.kind.as_str(),
                    report.width,
                    report.height,
                    report.hd_scale,
                    report.is_hd,
                    report.can_auto_upgrade,
                    report.description
                );
            } else {
                println!("File: {}", texture.display());
                println!("Kind: {}", report.kind.as_str());
                println!("Dimensions: {}x{}", report.width, report.height);
                println!("HD: {} (scale {}x)", report.is_hd, report.hd_scale);
                println!("Auto-upgradeable: {}", report.can_auto_upgrade);
                println!("Description: {}", report.description);
            }
            Ok(())
        }
    }
}
