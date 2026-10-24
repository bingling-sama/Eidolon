# Eidolon

[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

[简体中文](README_zh-CN.md)

Eidolon is a Minecraft skin renderer written in Rust. It can render a 3D model of a player skin to an image, or preview it in a window.

## Features

- Load and render Minecraft player models
- Texture mapping support
- 3D rendering with wgpu (Vulkan / Metal / DX12)
- Save static PNG/WebP or animated GIF/WebP output
- Configurable camera, poses, and output size
- Minecraft-style nametags above player models
- Cape and elytra back-equipment rendering
- Headless off-screen rendering and windowed preview
- Cross-platform: Windows, macOS, Linux

## Documentation

Full documentation lives in `docs/`.

- `docs/getting-started.md` Build, first render, and preview workflow
- `docs/cli.md` Complete command-line reference
- `docs/library.md` Library API usage patterns
- `docs/architecture.md` Project layout and render pipeline overview
- `docs/development.md` Local development, tests, and benchmarks
- `docs/troubleshooting.md` Common runtime and asset issues

## Quick Start

### Prerequisites

- Rust (latest stable recommended)
- A GPU with Vulkan, Metal, or DX12 support
- A working `wgpu` backend. On headless systems this may require a configured software backend such
  as OSMesa, depending on the platform.

### Build

```bash
cargo build
```

### Render an Image

```bash
# Minimal — all defaults (classic arms, idle pose, 800×600 PNG)
cargo run -- render resources/bingling_sama.png

# Slim arms, wave pose, WebP output
cargo run -- render resources/bingling_sama.png out.webp --slim --posture wave

# Custom camera angle and zoom
cargo run -- render resources/bingling_sama.png --cam-yaw 210 --cam-zoom 1.5 --width 1024 --height 1024

# Render a player nametag
cargo run -- render resources/bingling_sama.png --nametag BinglingTX

# Render a procedural animation
cargo run -- render resources/bingling_sama.png walk.gif --animate --posture walk
```

### Preview in a Window

```bash
cargo run -- preview resources/bingling_sama.png
cargo run -- preview resources/bingling_sama.png --slim --cam-zoom 2.0
```

More CLI options are documented in `docs/cli.md`.

## Library Usage

Eidolon can be used as a Rust library. Minimal example:

```rust
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    output::{write_output, OutputIntent, StaticFormat},
    renderer::Renderer,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;

    let body = renderer.load_body("skin.png", SkinType::Classic)?;
    let character = Character::from_body(body);
    let camera = Camera::new();

    write_output(
        "output.png",
        OutputIntent::Static {
            format: StaticFormat::Png,
            size: (800, 600),
        },
        |_: Duration| renderer.render(&character, &camera, 800, 600),
    )?;

    Ok(())
}
```

More details are in `docs/library.md`.

## Contributing

Contributions are welcome. See `docs/contributing.md`.

## License

MIT. See [LICENSE](LICENSE).

## Credits

See `docs/credits.md` and `docs/resources.md`.
