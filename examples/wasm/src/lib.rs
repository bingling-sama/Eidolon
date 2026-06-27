//! WASM bindings for eidolon — renders Minecraft skins in the browser via WebGPU.
//!
//! Build with `wasm-pack build --target bundler`, then import from JS.
//! Requires a browser with WebGPU support (Chrome 113+, Edge 113+, Firefox Nightly).

use eidolon::camera::Camera;
use eidolon::character::Character;
use eidolon::renderer::Renderer;
use image::ImageFormat;
use std::io::Cursor;
use wasm_bindgen::prelude::*;

/// Initialize `console_log` so Rust `log` macros print to the browser console.
#[wasm_bindgen(start)]
pub fn init_logger() {
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("eidolon WASM initialized");
}

/// Off-screen skin renderer backed by WebGPU.
#[wasm_bindgen]
pub struct SkinRenderer {
    inner: Renderer,
}

#[wasm_bindgen]
impl SkinRenderer {
    /// Create a headless renderer (async — call as `await SkinRenderer.new()`).
    pub async fn new() -> Result<SkinRenderer, JsValue> {
        let inner = Renderer::new_async()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        log::info!("Renderer created");
        Ok(SkinRenderer { inner })
    }

    /// Render a skin from PNG bytes, return the rendered image as PNG bytes.
    ///
    /// `skin_bytes` — raw PNG file content (e.g. from `fetch` or `FileReader`).
    /// Returns PNG bytes suitable for `URL.createObjectURL` or `<img src>`.
    #[wasm_bindgen]
    pub async fn render(&self, skin_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
        let texture = self
            .inner
            .load_texture_from_memory(skin_bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let character = Character::new();
        let camera = Camera::default();
        let (width, height) = (800, 600);

        let image_buffer = self
            .inner
            .render_async(&character, &texture, &camera, width, height)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mut png_bytes = Vec::new();
        image_buffer
            .write_to(
                &mut Cursor::new(&mut png_bytes),
                ImageFormat::Png,
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        log::info!("Rendered {}×{} → {} PNG bytes", width, height, png_bytes.len());
        Ok(png_bytes)
    }
}
