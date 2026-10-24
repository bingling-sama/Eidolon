//! WASM bindings for Eidolon via WebGPU.
//!
//! Build with `wasm-pack build --target bundler`, then import from JS.
//! Requires a browser with WebGPU support (Chrome 113+, Edge 113+, Firefox Nightly).

#![cfg(target_arch = "wasm32")]

pub mod classifier;
pub mod renderer;

pub use classifier::{inspect_texture, JsTextureReport};
pub use renderer::CanvasRenderer;

use wasm_bindgen::prelude::*;

/// Initialize `console_log` so Rust `log` macros print to the browser console.
#[wasm_bindgen(start)]
pub fn init_logger() {
    console_log::init_with_level(log::Level::Info).ok();
    console_error_panic_hook::set_once();
    log::info!("eidolon WASM initialized");
}
