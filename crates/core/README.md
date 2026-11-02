# eidolon-core

[![Crates.io](https://img.shields.io/crates/v/eidolon-core.svg)](https://crates.io/crates/eidolon-core)
[![Docs.rs](https://docs.rs/eidolon-core/badge.svg)](https://docs.rs/eidolon-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A high-performance Minecraft skin 3D rendering core engine written in Rust using [wgpu](https://github.com/gfx-rs/wgpu).

`eidolon-core` powers headless off-screen rendering, windowed interactive previewing, and WebAssembly in-browser rendering.

## Features

- **Player Geometry**: Accurate Steve (Classic, 4px arms) and Alex (Slim, 3px arms) dual-layer mesh models.
- **Hardware-Accelerated Rendering**: Built on `wgpu` (Vulkan, Metal, DirectX 12, WebGPU / WebGL).
- **Back Equipment**:
  - **Capes**: Minecraft-authentic 64×32 and HD capes with independent pitch adjustment.
  - **Elytra**: Dual-wing flight geometry with configurable opening angles and pitch.
- **Nametags**: Vanilla-faithful player nametag rendering with billboard alignment and transparent depth-pass occlusion.
- **Procedural Animations**: Built-in animation schedules (`idle`, `walk`, `run`, `wave`, `fly`) and fine-grained joint angle overrides.
- **Format Encoders**:
  - **Static**: PNG, WebP.
  - **Animated**: Animated GIF, animated WebP, APNG, AVIF.
- **Texture Analysis & Conversion**:
  - Automatic detection of modern 64×64 dual-layer skins, legacy 64×32 single-layer skins, and HD resolutions.
  - Lossless legacy single-layer to modern double-layer skin conversion.

## Installation

Add `eidolon-core` to your `Cargo.toml`:

```toml
[dependencies]
eidolon-core = "0.0.3-alpha"
```

### Feature Flags

- `winit` *(optional)*: Enables integration with `winit` for windowed rendering surfaces (`Renderer::new_windowed`).
- `webgl` *(optional)*: Enables WebGL fallback support for `wasm32-unknown-unknown` targets.

## Quick Start

### 1. Minimal Render to PNG

```rust
use eidolon_core::{
    camera::Camera,
    character::{Character, SkinType},
    output::{write_output, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GPU renderer
    let renderer = Renderer::new()?;

    // Load skin texture (auto-expands legacy 64x32 single-layer skins if needed)
    let body = renderer.load_body("skin.png", SkinType::Classic)?;
    let character = Character::from_body(body);
    let camera = Camera::new(); // Defaults: front view (yaw=180, pitch=90), zoom=1.0

    // Render off-screen directly to file
    write_output(
        "output.png",
        OutputIntent::Static {
            format: StaticFormat::Png,
            size: (800, 600),
        },
        |_: Duration| renderer.render(&character, &camera, 800, 600),
    )?;

    println!("Rendered output.png");
    Ok(())
}
```

### 2. Custom Poses, Nametag, and Cape

```rust
use eidolon_core::{
    character::{Character, Posture, SkinType},
    renderer::Renderer,
};

fn configure_character(renderer: &Renderer) -> Result<Character, Box<dyn std::error::Error>> {
    let body = renderer.load_body("skin.png", SkinType::Slim)?;
    let mut character = Character::from_body(body);

    // Set action pose (idle, walk, run, wave, fly)
    character.set_posture(Posture::Wave);

    // Attach a nametag above the head
    character.set_nametag(Some("Steve".to_string()));

    // Attach a cape
    let cape_atlas = renderer.load_back_equipment("cape.png")?;
    character.set_cape(Some(cape_atlas));

    Ok(character)
}
```

### 3. Rendering an Animated WebP or GIF

```rust
use eidolon_core::{
    animation::AnimationPreset,
    camera::Camera,
    character::Character,
    output::{write_output, AnimationFormat, OutputIntent},
    renderer::Renderer,
};
use std::time::Duration;

fn render_walk_cycle(
    renderer: &Renderer,
    character: &mut Character,
    camera: &Camera,
) -> Result<(), Box<dyn std::error::Error>> {
    let preset = AnimationPreset::Walk;

    write_output(
        "walk.webp",
        OutputIntent::Animation {
            format: AnimationFormat::Webp,
            size: (800, 600),
            fps: 30,
            speed: 1.0,
            loop_count: 0, // 0 = infinite loop
            preset,
        },
        |time: Duration| {
            character.apply_animation_schedule(preset, time);
            renderer.render(character, camera, 800, 600)
        },
    )?;

    Ok(())
}
```

## Documentation

For architectural details, advanced APIs, and guides, refer to the project documentation:
- [Library Guide](https://github.com/bingling-sama/eidolon/blob/main/docs/library.md)
- [Architecture Overview](https://github.com/bingling-sama/eidolon/blob/main/docs/architecture.md)
- [Command Line Interface (eidolon-cli)](https://github.com/bingling-sama/eidolon/blob/main/docs/cli.md)
- [Main Repository](https://github.com/bingling-sama/eidolon)

## License

Licensed under the [MIT License](https://github.com/bingling-sama/eidolon/blob/main/LICENSE).
