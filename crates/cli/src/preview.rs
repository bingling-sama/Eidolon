use std::sync::Arc;

use eidolon_core::{
    animation::{AnimationPreset, AnimationTimeline},
    back_equipment::BackEquipment,
    camera::Camera,
    character::{Character, Posture},
    error::EidolonError,
    renderer::Renderer,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::args::PostureCli;
use crate::scene::{PostureOverrides, SceneState};

pub struct PreviewApp {
    pub renderer: Option<Renderer>,
    pub window: Option<Arc<Window>>,
    pub character: Option<Character>,
    pub scene_state: SceneState,
    pub camera: Camera,
    pub texture_path: String,
    pub cape_atlas_path: Option<String>,
    pub elytra_atlas_path: Option<String>,
    pub initial_size: PhysicalSize<u32>,
    pub interaction: PreviewInteraction,
    pub playback: PreviewPlayback,
    pub posture_overrides: PostureOverrides,
}

pub struct PreviewPlayback {
    pub timeline: AnimationTimeline,
    pub last_tick: Option<std::time::Instant>,
}

impl PreviewPlayback {
    pub fn new(preset: AnimationPreset, playing: bool) -> Self {
        let mut timeline = AnimationTimeline::new(preset);
        if playing {
            timeline.play();
        }
        Self {
            timeline,
            last_tick: None,
        }
    }

    pub fn from_posture(posture: PostureCli) -> Self {
        let mut playback = Self::new(posture.animation_preset(), false);
        playback.timeline.seek_to(posture.sample_time());
        playback
    }

    pub fn tick(&mut self, now: std::time::Instant) {
        if let Some(previous) = self.last_tick {
            let delta = now.saturating_duration_since(previous);
            self.timeline.advance(delta);
        }
        self.last_tick = Some(now);
    }

    pub fn toggle(&mut self, now: std::time::Instant) {
        self.tick(now);
        if self.timeline.is_playing() {
            self.timeline.pause();
        } else {
            self.timeline.play();
        }
    }

    pub fn suspend(&mut self) {
        self.last_tick = None;
    }

    pub fn select(&mut self, preset: AnimationPreset, now: std::time::Instant) {
        self.timeline.set_preset(preset);
        self.last_tick = Some(now);
    }

    pub fn seek_frames(&mut self, frames: i32) {
        let delta = std::time::Duration::from_secs_f64(f64::from(frames.unsigned_abs()) / 30.0);
        if frames.is_negative() {
            self.timeline.seek_back(delta);
        } else {
            self.timeline.seek_by(delta);
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.timeline.set_speed(speed);
    }

    pub fn adjust_speed(&mut self, factor: f32) {
        self.set_speed(self.timeline.speed() * factor);
    }

    #[cfg(test)]
    pub fn elapsed(&self) -> std::time::Duration {
        self.timeline.position()
    }

    pub fn sample(&self) -> Posture {
        self.timeline.sample()
    }
}

#[derive(Default)]
pub struct PreviewInteraction {
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub last_cursor: Option<(f64, f64)>,
    pub pan_x: f32,
    pub pan_y: f32,
    pub base_position: Option<cgmath::Vector3<f32>>,
}

impl PreviewInteraction {
    pub fn mouse_input(&mut self, button: MouseButton, pressed: bool) {
        if button == MouseButton::Left {
            self.left_pressed = pressed;
        } else if button == MouseButton::Right {
            self.right_pressed = pressed;
        }
    }

    pub fn cursor_moved(
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

    pub fn scroll(&mut self, delta: f32, position: &mut cgmath::Vector3<f32>, camera: &mut Camera) {
        self.base_position.get_or_insert(*position);
        camera.scale = (camera.scale * 1.1_f32.powf(delta)).clamp(0.1, 10.0);
        self.update_pan(position, camera);
    }

    pub fn update_pan(&self, position: &mut cgmath::Vector3<f32>, camera: &Camera) {
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
                        Err(EidolonError::Surface(
                            eidolon_core::error::SurfaceError::Lost
                            | eidolon_core::error::SurfaceError::Outdated,
                        )) => {
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                if let Some(r) = &mut self.renderer {
                                    r.resize(size.width, size.height);
                                }
                            }
                        }
                        Err(EidolonError::Surface(
                            eidolon_core::error::SurfaceError::OutOfMemory,
                        )) => {
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

    #[test]
    fn preview_posture_selects_the_animation_used_when_playback_starts() {
        let args = crate::args::Args::try_parse_from([
            "eidolon",
            "preview",
            "skin.png",
            "--posture",
            "fly",
        ])
        .expect("flying preview should parse");

        let crate::args::Command::Preview { scene, .. } = args.command else {
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
}
