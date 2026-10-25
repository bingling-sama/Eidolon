use std::io::Cursor;
use std::time::Duration;

use eidolon_core::animation::{AnimationPreset, AnimationTimeline, LoopMode};
use eidolon_core::back_equipment::{BackEquipment, BackEquipmentKind};
use eidolon_core::camera::Camera;
use eidolon_core::character::{Character, DefaultPostures, Posture, SkinType};
use eidolon_core::renderer::Renderer;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::ImageFormat;
use wasm_bindgen::prelude::*;

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
    timeline: AnimationTimeline,
    elytra_override: Option<(f32, f32)>,
    nametag: Option<String>,
    nametag_see_through: bool,
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
            timeline: AnimationTimeline::new(AnimationPreset::Idle),
            elytra_override: None,
            nametag: None,
            nametag_see_through: true,
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
            character.nametag = self.nametag.clone();
            character.nametag_see_through = self.nametag_see_through;
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
            character.nametag = self.nametag.clone();
            character.nametag_see_through = self.nametag_see_through;
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

    /// Disable skin/body rendering.
    pub fn clear_skin(&mut self) {
        let only_body = self
            .character
            .as_ref()
            .is_some_and(|c| c.back_equipment().is_none());
        if only_body {
            self.pending_posture = self.character.as_ref().map(|c| c.posture);
            self.character = None;
        } else if let Some(character) = &mut self.character {
            let _ = character.remove_body();
        }
        log::info!("Skin cleared");
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
            character.nametag = self.nametag.clone();
            character.nametag_see_through = self.nametag_see_through;
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
        self.timeline.set_preset(parse_animation(preset)?);
        self.apply_animation_sample();
        Ok(())
    }

    /// Seek the selected animation to an explicit position in seconds.
    ///
    /// Kept for compatibility with older browser clients; new clients should
    /// use `advance_animation` and the playback controls below.
    pub fn sample_animation(&mut self, time_seconds: f64) -> Result<(), JsValue> {
        self.timeline.seek_to(duration_from_seconds(
            time_seconds,
            "animation time must be finite, non-negative, and representable",
        )?);
        self.apply_animation_sample();
        Ok(())
    }

    /// Natural cycle duration of the selected animation, in seconds.
    pub fn animation_duration(&self) -> f64 {
        self.timeline.duration().as_secs_f64()
    }

    /// Advance playback by a browser-provided frame delta in seconds.
    pub fn advance_animation(&mut self, delta_seconds: f64) -> Result<(), JsValue> {
        self.timeline.advance(duration_from_seconds(
            delta_seconds,
            "animation delta must be finite, non-negative, and representable",
        )?);
        self.apply_animation_sample();
        Ok(())
    }

    /// Start or pause playback.
    pub fn set_animation_playing(&mut self, playing: bool) {
        if playing {
            self.timeline.play();
        } else {
            self.timeline.pause();
        }
    }

    /// Set playback speed; values are clamped to the core timeline range.
    pub fn set_animation_speed(&mut self, speed: f32) {
        self.timeline.set_speed(speed);
    }

    /// Select looping (`true`) or one-shot (`false`) playback.
    pub fn set_animation_loop(&mut self, looping: bool) {
        self.timeline.set_loop_mode(if looping {
            LoopMode::Loop
        } else {
            LoopMode::Once
        });
        self.apply_animation_sample();
    }

    /// Current playback position in seconds.
    pub fn animation_time(&self) -> f64 {
        self.timeline.position().as_secs_f64()
    }

    /// Whether the timeline is currently playing.
    pub fn animation_playing(&self) -> bool {
        self.timeline.is_playing()
    }

    /// Whether a one-shot timeline has reached its final pose.
    pub fn animation_finished(&self) -> bool {
        self.timeline.is_finished()
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

    /// Set or clear the player nametag. An empty string or whitespace removes the nametag.
    pub fn set_nametag(&mut self, text: &str) {
        let nametag = if text.trim().is_empty() {
            None
        } else {
            Some(text.to_owned())
        };
        self.nametag = nametag.clone();
        if let Some(character) = &mut self.character {
            character.nametag = nametag;
        }
    }

    /// Remove the player nametag.
    pub fn clear_nametag(&mut self) {
        self.nametag = None;
        if let Some(character) = &mut self.character {
            character.nametag = None;
        }
    }

    /// Set whether occluded nametag pixels remain visible at reduced opacity.
    pub fn set_nametag_see_through(&mut self, see_through: bool) {
        self.nametag_see_through = see_through;
        if let Some(character) = &mut self.character {
            character.nametag_see_through = see_through;
        }
    }

    /// Current nametag text, or `None` if hidden.
    pub fn nametag(&self) -> Option<String> {
        self.character
            .as_ref()
            .and_then(|character| character.nametag.clone())
            .or_else(|| self.nametag.clone())
    }

    /// Whether nametag see-through occlusion is enabled.
    pub fn nametag_see_through(&self) -> bool {
        self.character
            .as_ref()
            .map(|character| character.nametag_see_through)
            .unwrap_or(self.nametag_see_through)
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

    /// Offscreen render at high resolution, return encoded image bytes for download.
    /// Renders at the given size independently of the canvas.
    /// Supports "png" (default), "webp", and "jpeg"/"jpg".
    pub async fn capture_frame(
        &self,
        width: u32,
        height: u32,
        format: Option<String>,
    ) -> Result<Vec<u8>, JsValue> {
        let character = self
            .character
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no character resources loaded"))?;
        let image_buffer = self
            .inner
            .render_async(character, &self.camera, width, height)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let format_str = format.as_deref().unwrap_or("png");
        let mut encoded_bytes = Vec::new();
        match format_str.to_ascii_lowercase().as_str() {
            "png" => {
                let encoder = PngEncoder::new_with_quality(
                    &mut encoded_bytes,
                    CompressionType::Fast,
                    FilterType::Sub,
                );
                image_buffer
                    .write_with_encoder(encoder)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
            }
            "webp" => {
                image_buffer
                    .write_to(&mut Cursor::new(&mut encoded_bytes), ImageFormat::WebP)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
            }
            "jpeg" | "jpg" => {
                let rgb = image::DynamicImage::ImageRgba8(image_buffer).to_rgb8();
                let encoder = JpegEncoder::new_with_quality(&mut encoded_bytes, 80);
                rgb.write_with_encoder(encoder)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
            }
            other => return Err(JsValue::from_str(&format!("unsupported format: '{other}'"))),
        }
        log::info!(
            "Captured {}×{} ({}) → {} bytes",
            width,
            height,
            format_str,
            encoded_bytes.len()
        );
        Ok(encoded_bytes)
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
        let mut posture = self.timeline.sample();
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
        other => Err(JsValue::from_str(&format!("unknown animation: '{other}'"))),
    }
}

fn duration_from_seconds(seconds: f64, message: &str) -> Result<Duration, JsValue> {
    Duration::try_from_secs_f64(seconds).map_err(|_| JsValue::from_str(message))
}
