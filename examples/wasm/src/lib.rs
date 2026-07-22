//! WASM bindings for eidolon — renders Minecraft skins in the browser via WebGPU.
//!
//! Build with `wasm-pack build --target bundler`, then import from JS.
//! Requires a browser with WebGPU support (Chrome 113+, Edge 113+, Firefox Nightly).

use eidolon::back_equipment::{BackEquipment, BackEquipmentKind};
use eidolon::animation::AnimationPreset;
use eidolon::camera::Camera;
use eidolon::character::{Character, DefaultPostures, Posture, SkinType};
use eidolon::renderer::Renderer;
use image::ImageFormat;
use std::io::Cursor;
use std::time::Duration;
use wasm_bindgen::prelude::*;

/// Initialize `console_log` so Rust `log` macros print to the browser console.
#[wasm_bindgen(start)]
pub fn init_logger() {
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("eidolon WASM initialized");
}

/// Real-time canvas renderer — presents directly to a `<canvas>` element at 60fps.
///
/// No readback, no PNG encode — GPU renders straight to the canvas swapchain.
#[wasm_bindgen]
pub struct CanvasRenderer {
    inner: Renderer,
    character: Option<Character>,
    pending_posture: Option<Posture>,
    skin_type: SkinType,
    camera: Camera,
    animation: AnimationPreset,
    animation_time: Duration,
    elytra_override: Option<(f32, f32)>,
}

#[wasm_bindgen]
impl CanvasRenderer {
    /// Attach to a `<canvas>` element. Call as `await CanvasRenderer.new(canvas)`.
    pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<CanvasRenderer, JsValue> {
        let inner = Renderer::new_windowed_async(wgpu::SurfaceTarget::Canvas(canvas))
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        log::info!("Canvas renderer created");
        Ok(CanvasRenderer {
            inner,
            character: None,
            pending_posture: Some(DefaultPostures::STAND),
            skin_type: SkinType::Classic,
            camera: Camera::default(),
            animation: AnimationPreset::Idle,
            animation_time: Duration::ZERO,
            elytra_override: None,
        })
    }

    /// Load a skin from PNG bytes. Must be called before `render_frame`.
    pub fn load_skin(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let body = self
            .inner
            .load_body_from_memory(bytes, self.skin_type)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if let Some(character) = &mut self.character {
            character.attach_body(body);
        } else {
            let mut character = Character::from_body(body);
            character.posture = self
                .pending_posture
                .take()
                .unwrap_or(DefaultPostures::STAND);
            self.character = Some(character);
        }
        log::info!("Skin loaded");
        Ok(())
    }

    /// Load a cape from PNG bytes and enable cape rendering.
    pub fn load_cape(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let atlas = self
            .inner
            .load_back_equipment_atlas_from_memory(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let equipment = BackEquipment::new(atlas, BackEquipmentKind::Cape);
        if let Some(character) = &mut self.character {
            character.equip(equipment);
        } else {
            let mut character = Character::from_back_equipment(equipment);
            character.posture = self
                .pending_posture
                .take()
                .unwrap_or(DefaultPostures::STAND);
            self.character = Some(character);
        }
        log::info!("Cape loaded");
        Ok(())
    }

    /// Disable cape rendering.
    pub fn clear_cape(&mut self) {
        self.clear_back_equipment(BackEquipmentKind::Cape);
        log::info!("Cape cleared");
    }

    /// Load a Back equipment atlas from PNG bytes and select Elytra presentation.
    pub fn load_elytra(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let atlas = self
            .inner
            .load_back_equipment_atlas_from_memory(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let equipment = BackEquipment::new(atlas, BackEquipmentKind::Elytra);
        if let Some(character) = &mut self.character {
            character.equip(equipment);
        } else {
            let mut character = Character::from_back_equipment(equipment);
            character.posture = self
                .pending_posture
                .take()
                .unwrap_or(DefaultPostures::STAND);
            self.character = Some(character);
        }
        log::info!("Elytra loaded");
        Ok(())
    }

    /// Disable elytra rendering.
    pub fn clear_elytra(&mut self) {
        self.clear_back_equipment(BackEquipmentKind::Elytra);
        log::info!("Elytra cleared");
    }

    /// Set posture by preset. `"stand"`, `"wave"`, `"walking"`, `"running"`, `"flying"`.
    pub fn set_posture(&mut self, preset: &str) -> Result<(), JsValue> {
        let posture = match preset {
            "stand" => DefaultPostures::STAND,
            "wave" => DefaultPostures::WAVE,
            "walking" => DefaultPostures::WALKING,
            "running" => DefaultPostures::RUNNING,
            "flying" => DefaultPostures::FLYING,
            other => return Err(JsValue::from_str(&format!("unknown posture: '{other}'"))),
        };
        if let Some(character) = &mut self.character {
            character.posture = posture;
        } else {
            self.pending_posture = Some(posture);
        }
        Ok(())
    }

    /// Select a procedural animation and reset it to phase zero.
    pub fn set_animation(&mut self, preset: &str) -> Result<(), JsValue> {
        self.animation = parse_animation(preset)?;
        self.animation_time = Duration::ZERO;
        self.apply_animation_sample();
        Ok(())
    }

    /// Sample the selected animation at a JS-owned playback time in seconds.
    pub fn sample_animation(&mut self, time_seconds: f64) -> Result<(), JsValue> {
        if !time_seconds.is_finite() || time_seconds < 0.0 {
            return Err(JsValue::from_str("animation time must be finite and non-negative"));
        }
        self.animation_time = Duration::from_secs_f64(time_seconds);
        self.apply_animation_sample();
        Ok(())
    }

    /// Natural cycle duration of the selected animation, in seconds.
    pub fn animation_duration(&self) -> f64 {
        self.animation.cycle_duration().as_secs_f64()
    }

    /// Override elytra pitch and opening angle in degrees.
    pub fn set_elytra_angles(&mut self, pitch: f32, spread: f32) {
        self.elytra_override = Some((pitch, spread));
        self.apply_animation_sample();
    }

    /// Remove the manual elytra override and return control to the animation.
    pub fn reset_elytra_angles(&mut self) {
        self.elytra_override = None;
        self.apply_animation_sample();
    }

    /// Current elytra pitch in degrees.
    pub fn elytra_pitch(&self) -> f32 {
        self.character
            .as_ref()
            .map(|character| character.posture.elytra_pitch)
            .or_else(|| self.pending_posture.map(|posture| posture.elytra_pitch))
            .unwrap_or(DefaultPostures::STAND.elytra_pitch)
    }

    /// Current elytra opening angle in degrees.
    pub fn elytra_spread(&self) -> f32 {
        self.character
            .as_ref()
            .map(|character| character.posture.elytra_roll)
            .or_else(|| self.pending_posture.map(|posture| posture.elytra_roll))
            .unwrap_or(DefaultPostures::STAND.elytra_roll)
    }

    /// Set arm width. `"classic"` (4px) or `"slim"` (3px).
    pub fn set_skin_type(&mut self, skin_type: &str) -> Result<(), JsValue> {
        self.skin_type = match skin_type {
            "classic" => SkinType::Classic,
            "slim" => SkinType::Slim,
            other => return Err(JsValue::from_str(&format!("unknown skin type: '{other}'"))),
        };
        if let Some(body) = self.character.as_mut().and_then(Character::body_mut) {
            body.set_skin_type(self.skin_type);
        }
        Ok(())
    }

    /// Set camera orbit. `yaw`/`pitch` in degrees, `scale` > 0 (higher = closer).
    pub fn set_camera(&mut self, yaw: f32, pitch: f32, scale: f32) {
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;
        self.camera.scale = scale;
    }

    /// Set background clear color. `r`,`g`,`b`,`a` in 0.0-1.0 range.
    /// Default is transparent black `(0,0,0,0)`.
    pub fn set_background(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.inner.set_clear_color(r, g, b, a);
    }

    /// Update surface size after canvas resize. Call from ResizeObserver.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.inner.resize(width, height);
    }

    /// Offscreen render at high resolution, return PNG bytes for download.
    /// Renders at the given size independently of the canvas.
    pub async fn capture_frame(&self, width: u32, height: u32) -> Result<Vec<u8>, JsValue> {
        let character = self
            .character
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no character resources loaded"))?;
        let image_buffer = self
            .inner
            .render_async(character, &self.camera, width, height)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut png_bytes = Vec::new();
        image_buffer
            .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        log::info!("Captured {}×{} → {} bytes", width, height, png_bytes.len());
        Ok(png_bytes)
    }

    /// Present one frame to the canvas. Call in a rAF loop at 60fps.
    pub fn render_frame(&self) -> Result<(), JsValue> {
        let character = self
            .character
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no character resources loaded"))?;
        self.inner
            .render_frame(character, &self.camera)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        Ok(())
    }
}

impl CanvasRenderer {
    fn apply_animation_sample(&mut self) {
        let mut posture = self.animation.sample(self.animation_time);
        if let Some((pitch, spread)) = self.elytra_override {
            posture.elytra_pitch = pitch;
            posture.elytra_roll = spread;
        }
        if let Some(character) = &mut self.character {
            character.posture = posture;
        } else {
            self.pending_posture = Some(posture);
        }
    }

    fn clear_back_equipment(&mut self, kind: BackEquipmentKind) {
        let should_clear = self
            .character
            .as_ref()
            .and_then(Character::back_equipment)
            .is_some_and(|equipment| equipment.kind() == kind);
        if !should_clear {
            return;
        }

        let only_back_equipment = self
            .character
            .as_ref()
            .is_some_and(|character| character.body().is_none());
        if only_back_equipment {
            self.pending_posture = self.character.as_ref().map(|character| character.posture);
            self.character = None;
        } else if let Some(character) = &mut self.character {
            character
                .unequip()
                .expect("body-backed character can remove back equipment");
        }
    }
}

fn parse_animation(preset: &str) -> Result<AnimationPreset, JsValue> {
    match preset {
        "idle" => Ok(AnimationPreset::Idle),
        "walk" => Ok(AnimationPreset::Walk),
        "run" => Ok(AnimationPreset::Run),
        "wave" => Ok(AnimationPreset::Wave),
        "fly" => Ok(AnimationPreset::Fly),
        other => Err(JsValue::from_str(&format!(
            "unknown animation: '{other}'"
        ))),
    }
}
