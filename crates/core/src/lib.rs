/*!
Eidolon: Minecraft skin rendering library.

Provides:
- Loading and rendering Minecraft skins (PNG; single-layer skins are expanded to double-layer when needed)
- Configurable character posture and camera
- Minecraft-style player nametags
- Headless image output (PNG / WebP / GIF / Animated WebP) and windowed preview
*/

#![cfg_attr(target_arch = "wasm32", allow(clippy::arc_with_non_send_sync))]

pub mod animation;
pub mod assets;
pub mod back_equipment;
pub mod body;
pub mod camera;
pub mod character;
pub mod classifier;
pub mod constants;
pub mod converter;
pub mod error;
pub mod model;
pub mod output;
pub mod renderer;
#[cfg(test)]
mod test_support;
mod texture;
