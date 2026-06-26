/*!
Eidolon: Minecraft skin rendering library.

Provides:
- Loading and rendering Minecraft skins (PNG; single-layer skins are expanded to double-layer when needed)
- Configurable character posture and camera
- Headless image output (PNG / WebP) and windowed preview
*/

pub mod camera;
pub mod character;
pub mod constants;
pub mod converter;
pub mod error;
pub mod model;
pub mod renderer;
pub mod texture;

pub use renderer::OutputFormat;
