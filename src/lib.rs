/*!
Eidolon: Minecraft skin rendering library.

Provides:
- Loading and rendering Minecraft skins (PNG; single-layer skins are expanded to double-layer when needed)
- Configurable character posture and camera
- Minecraft-style player nametags
- Headless image output (PNG / WebP / GIF / Animated WebP) and windowed preview
*/

pub mod animation;
pub mod back_equipment;
pub mod body;
pub mod camera;
mod cape;
pub mod character;
pub mod constants;
pub mod converter;
mod elytra;
pub mod error;
pub mod model;
pub mod output;
pub mod renderer;
mod texture;
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub mod wasm;
