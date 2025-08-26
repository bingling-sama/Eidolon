# Eidolon

[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

Eidolon is a Minecraft skin renderer written in Rust. It can render a 3D model of a player skin and save it as a PNG image.

## Features

- Load and render Minecraft player models
- Texture mapping support
- 3D rendering
- Save rendering result as a PNG image
- Configurable camera and output size

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended)
- **System dependencies:**  
	This project requires `osmesa`, which is included in `mesa` versions prior to 25.10.  
	- On Linux, install with your package manager (e.g., `sudo apt install libosmesa6-dev` on Ubuntu).
	- On other platforms, ensure you have a compatible version of `mesa` or `osmesa` available.
- **Note:** If you encounter build issues related to OpenGL or `osmesa`, check your system's `mesa` version and consider downgrading if necessary.

## Build and Run

1. Clone the repository:
    ```bash
    git clone https://github.com/bingling-sama/SkinViewer.git
    cd SkinViewer
    ```

2. Run with CLI subcommands:

### Render 3D Skin Image

Render a Minecraft skin as a 3D image.

```bash
cargo run -- render [OPTIONS]
```

**Options:**
- `--filename <FILENAME>`: Output image filename (default: `output.png`)
- `--width <WIDTH>`: Output image width (default: `800`)
- `--height <HEIGHT>`: Output image height (default: `600`)
- `--skin-type <SkinType>`: Skin type (`classic` or `slim`), required
- `--texture <TEXTURE>`: PNG skin file path (default: `resources/bingling_sama.png`)
- More options see `cargo run -- render --help`

**Example:**
```bash
cargo run -- render --filename my_skin.png --texture resources/bingling_sama.png --skin-type Steve --width 1024 --height 768 --yaw 180 --pitch 90 --scale 1.2
```

### Convert Single-layer Skin to Double-layer

Convert a classic single-layer Minecraft skin to double-layer format.

```bash
cargo run -- convert <INPUT> <OUTPUT>
```
- `<INPUT>`: Path to the single-layer skin PNG file
- `<OUTPUT>`: Path to save the converted double-layer PNG file

**Example:**
```bash
cargo run -- convert old_skin.png new_skin.png
```

## Library Usage

Eidolon can also be used as a Rust library for Minecraft skin rendering and image generation.

### Example

```rust
use skinviewer::{Renderer, Character, Camera};

let renderer = Renderer::new();
let mut character = Character::new();
// character.load_skin("path/to/skin.png");
let camera = Camera::new();
renderer.render_to_image(&character, &camera, "output.png", (800, 600));
```

### Main Components

- `Renderer`: Handles the rendering process and output.
- `Character`: Represents the Minecraft player model and skin.
- `Camera`: Controls the viewpoint, yaw, pitch, and scale.

See the source code and module docs for advanced usage, such as custom poses, camera settings, and texture loading.

## Dependencies

This project uses the following main crates:

- `glium`: For OpenGL rendering.
- `glutin`: For windowing and input.
- `image`: For image processing.
- `cgmath`: For 3D math.
- `tobj`: For loading `.obj` files.
- `clap`: For command-line argument parsing.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## Thanks

- Thanks to [@tnqzh123](https://github.com/tnqzh123) for project feature design and some technical support.
- Thanks to [@beanflame](https://github.com/beanflame) for opengl technical support.
- Thanks to [@sunjunnan79](https://github.com/sunjunnan79) for deleting `.DS_Store`. (Seriously)
- Thanks to [Blockbench](https://www.blockbench.net/) for providing the player model.
- Thanks to players who provided skins for testing.
