//! WASM bindings for eidolon — renders Minecraft skins in the browser via WebGPU.
//!
//! Build with `wasm-pack build --target bundler`, then import from JS.
//! Requires a browser with WebGPU support (Chrome 113+, Edge 113+, Firefox Nightly).

use eidolon::camera::Camera;
use eidolon::character::{Character, DefaultPostures, SkinType};
use eidolon::renderer::Renderer;
use eidolon::texture::Texture;
use image::ImageFormat;
use std::io::Cursor;
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
    skin: Option<Texture>,
    character: Character,
    camera: Camera,
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
            skin: None,
            character: Character::new(),
            camera: Camera::default(),
        })
    }

    /// Load a skin from PNG bytes. Must be called before `render_frame`.
    pub fn load_skin(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let texture = self
            .inner
            .load_texture_from_memory(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.skin = Some(texture);
        log::info!("Skin loaded");
        Ok(())
    }

    /// Set posture by preset. `"stand"`, `"wave"`, `"walking"`, `"running"`.
    pub fn set_posture(&mut self, preset: &str) -> Result<(), JsValue> {
        self.character.posture = match preset {
            "stand" => DefaultPostures::STAND,
            "wave" => DefaultPostures::WAVE,
            "walking" => DefaultPostures::WALKING,
            "running" => DefaultPostures::RUNNING,
            other => return Err(JsValue::from_str(&format!("unknown posture: '{other}'"))),
        };
        Ok(())
    }

    /// Set arm width. `"classic"` (4px) or `"slim"` (3px).
    pub fn set_skin_type(&mut self, skin_type: &str) -> Result<(), JsValue> {
        self.character.skin_type = match skin_type {
            "classic" => SkinType::Classic,
            "slim" => SkinType::Slim,
            other => return Err(JsValue::from_str(&format!("unknown skin type: '{other}'"))),
        };
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
        let skin = self
            .skin
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no skin loaded"))?;
        let image_buffer = self
            .inner
            .render_async(&self.character, skin, &self.camera, width, height)
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
        let skin = self
            .skin
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no skin loaded"))?;
        self.inner
            .render_frame(&self.character, skin, &self.camera)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        Ok(())
    }
}
