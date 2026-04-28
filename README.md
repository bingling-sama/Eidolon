# Eidolon

[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

[简体中文](README_zh-CN.md)

Eidolon is a Minecraft skin renderer written in Rust. It can render a 3D model of a player skin to an image, or preview it in a window.

## Features

- Load and render Minecraft player models
- Texture mapping support
- 3D rendering with wgpu (Vulkan / Metal / DX12)
- Save rendering result as PNG or WebP
- Configurable camera, poses, and output size
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
cargo run -- render --skin-type classic
```

### Preview in a Window

```bash
cargo run -- preview --skin-type classic
```

More CLI options are documented in `docs/cli.md`.

## Library Usage

Eidolon can be used as a Rust library. Minimal example:

```rust
use eidolon::{
    camera::Camera,
    character::{Character, SkinType},
    renderer::{OutputFormat, Renderer},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::new()?;
    let mut character = Character::new();
    character.skin_type = SkinType::Classic;
    character.skin = Some(renderer.load_texture("resources/bingling_sama.png")?);

    let camera = Camera::new();
    renderer.render_to_image(
        &character,
        &camera,
        "output.png",
        (800, 600),
        OutputFormat::Png,
    )?;

    Ok(())
}
```

More details are in `docs/library.md`.

## Contributing

Contributions are welcome. See `docs/contributing.md`.

## License

See `LICENSE`.

## Credits

See `docs/credits.md` and `docs/resources.md`.
